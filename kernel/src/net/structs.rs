use crate::arch::rand;
use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};
use crate::sync::SpinNoIrqLock as Mutex;
use crate::syscall::*;
use crate::util;
use alloc::boxed::Box;
use alloc::fmt::Debug;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use core::cmp::min;
use core::mem::size_of;
use core::slice;

use smoltcp::socket::*;
use smoltcp::wire::*;

#[derive(Clone, Debug)]
pub struct LinkLevelEndpoint {
    pub interface_index: usize,
}

impl LinkLevelEndpoint {
    pub fn new(ifindex: usize) -> Self {
        LinkLevelEndpoint {
            interface_index: ifindex,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetlinkEndpoint {
    pub port_id: u32,
    pub mulyicast_groups_mask: u32,
}

impl NetlinkEndpoint {
    pub fn new(port_id: u32, mulyicast_groups_mask: u32) -> Self{
        NetlinkEndpoint{
            port_id,
            mutticast_groups_mask,
        }
    }
}

#[derive(Clone,Debug)]
pub enum Endpoint{
    Ip(IpEndpoint),
    LinkLevelEndpoint(LinkLevelEndpoint), 
    NetlinkEndpoint(NetlinkEndpoint), 
}

/// Common  methods that a socket must have
pub trait Socket: Send  + Sync + Debug{
    fn read(&self, buf: &mut [u8]) -> (SysResult,Endpoint);
    fn write(&self, data: &[u8],sento_endpoint: Option<Endpoint>) -> SysResult;
    fn poll(&self) ->(bool,bool,bool);//(in,out,err)
    fn connect(&mut self,endpoint:Endpoint) -> SysResult;
    fn bind(&mut self,_endpoint:Endpoint)->SysResult{
        Err(SysError::EINVAL)
    }
    fn listen(&mut self) -> SysResult{
        Err(SysError::EINVAL)
    }
    fn shutdown(&mut self) -> SysResult{
        Err(SysError::EINVAL)
    }
    fn accpet(&mut self) ->Result<(Box<dyn Socket>,Endpoint),SysError>{
        Err(SysError::EINVAL)
    }
    fn endpoint(&mut self) -> Option<Endpoint>{
        None
    }
    fn remote_endpoint(&mut self) -> Option<Endpoint>{
        None
    }
    fn setsockopt(&mut self,_level:usize,_opt:usize,_data:&[u8]) -> SysResult{
        warn!("set sockopt is unimplemented");
        Ok(0)
    }
    fn ioctl(&mut self,_request :usize,_arg1:usize,_arg2:usize,_arg3:usize) -> SysResult{
        warn!("ioctl is not implemented for this socket")
        Ok(0)
    }
    fn box_clone(&self) -> Box<dyn Socket>;
}

impl Clone for Box<dyn Socket> {
    fn clone(&self) -> Box<dyn Socket> {
        self.box_clone()
    }
}

lazy_static! {
    ///Global SocketSet is smoltcp
    /// Becacuse smoltcp is a single thread network stack,
    /// every socket operation needs to lock this.
    pub static ref SOCKETS: Mutex<SocketSet<'static, 'static, 'static>> =
    Mutex::new(SocketSet::new(vec![]));
}

#[derive(Debug,Clone)]
pub struct TcpSocketState{
    handle: GlobalSocketHandle,
    local_endpoint:Option<IpEndpoint>,//save local endpoint for binding  ()
    is_listening:bool,
}

#[derive(Debug,Clone)]
pub struct UdpSocketState{
    handle : GlobalSocketHandle,
    remote_endpoint : Option<IpEndpoint>, // remeber remote endpoint for connect()
}

#[derive(Debug,Clone)]
pub struct RawSocketState{
    handle:GlobalSocketHandle,
    hander_included : bool,
}

#[derive(Debug,Clone)]
pub struct PacketSocketState{
    // no state , only ethernet  egress
}

#[derive(Debug,Clone)]
pub struct NetLinkSocketState{
    data: Arc<Mutex<Vec<Vec<u8>>>>,
}

/// A wrapper for 'SocketHandle'.
/// Auto increse and decrese reference count on Clone and Drop
#[derive(Debug)]
struct GlobalSocketHandle(SocketHandle);

impl Clone for GlobalSocketHandle {
    fn clone(&self) -> Self {
        SOCKETS.lock().retain(self.0);
        Self(self.0)
    }
}

impl Drop for GlobalSocketHandle {
    fn drop(&mut self) {
         let mut sockets = SOCKETS.lock();
         socket.release(self.0);
         socket.prune();

         /// send FIN immediately when applicable
         drop(sockets);
         poll_ifaces();
    }
}

impl TcpSocketState{
    pub fn new() ->Self{
        let rx_buffer = TcpSocketBuffer::new(vec![0;TCP_RECVBUF]);
        let tx_buffer = TcpSocketBuffer::new(vec![0;TCP_SENDBUF]);
        let socket = TcpSocket::new(rx_buffer, tx_buffer);
        let handle  = GlobalSocketHandle(SOCKETS.lock().add(socket));

        TcpSocketState{
            handle,
            local_endpoint: None,
            is_listening: false,
        }
    }
}

impl Socket for TcpSocketState {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint) {
        spin_and_wait(&[&SOCKET_ACTIVITY], move || {
            poll_ifaces();
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<TcpSocket>(self.handle.0);

            if socket.may_recv() {
                if let Ok(size) = socket.recv_slice(data) {
                    if size > 0 {
                        let endpoint = socket.remote_endpoint();
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        poll_ifaces();
                        return Some((Ok(size), Endpoint::Ip(endpoint)));
                    }
                }
            } else {
                return Some((
                    Err(SysError::ENOTCONN),
                    Endpoint::Ip(IpEndpoint::UNSPECIFIED),
                ));
            }
            None
        })
    }

    fn write(&self, data: &[u8], _sendto_endpoint: Option<Endpoint>) -> SysResult {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);

        if socket.is_open() {
            if socket.can_send() {
                match socket.send_slice(&data) {
                    Ok(size) => {
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        poll_ifaces();
                        Ok(size)
                    }
                    Err(_) => Err(SysError::ENOBUFS),
                }
            } else {
                Err(SysError::ENOBUFS)
            }
        } else {
            Err(SysError::ENOTCONN)
        }
    }

    fn poll(&self) -> (bool,bool,bool) {
        let mut sockets  = SOCKETS.lock();
        let socket = sockets.get::<TcpSocket>(self.handle.0);

        let(mut input,mut output, mut err) = (false,false,false);
        if self.is_listening && socket.is_active() {
            // a new connection is established
            input = true;
        }else if !socket.is_open() {
            err = true;
        }else{
            if socket.can.recv(){
                input = true;
            }
            if socket.can.send(){
                output = true;
            }
        }
        (input,output,err)
    }   

    fn connect(&mut self,endpoint : Endpoint)->SysResult{
        let mut sockets  = SOCKETS.lock();
        let mut socket  = sockets.get::<TcpSocket>(self.handle.0);

        if let Endpoint::Ip(ip)  = endpoint{
            let temp_port  = get_ephemeral_port();

            match socket.connect(ip, temp_port){
                Ok(()) =>{
                    // avoid deadlock
                    drop(Socket);
                    drop(sockets);

                    // wait for connection result 
                    loop{
                        poll_ifaces();

                        let mut sockets  = SOCKETS.lock();
                        let socket = sockets.get::<TcpSocket>(self.handle.0);
                        match socket.state{
                            TcpState::SynSent =>{
                                //still connnecting 
                                drop(socket);
                                debug!("poll for connection wait");
                                SOCKET_ACTIVITY.wait(sockets);
                            }
                            TcpState::established =>{
                                break Ok(0);
                            }
                            _ =>{
                                break Err(SysError::ECONNREFUSED); 
                            }
                        }
                    }
                }
                Err(_) => Err(SysError::ENOBUFS);
            }
        }else{
            Err(SysError::EINVAL)
        }
    }

    fn bind(&mut self, endpoint: Endpoint) -> SysResult{
        if let Endpoint::Ip(mut ip)  = endpoint{
            if ip,port == 0{
                ip.port = get_ephemeral_port();
            }
            self.local_endpoint = Some(ip);
            self.is_listening = false;
            Ok(0)
        }else{
            Err(SysError::EINVAL)
        }
    }

    fn listen(&mut self) -> SysResult {
        if self.is_listening {
            // it is ok to listen twice
            return Ok(0);
        }
        let local_endpoint = self.local_endpoint.ok_or(SysError::EINVAL)?;
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);

        info!("socket listening on {:?}", local_endpoint);
        if socket.is_listening() {
            return Ok(0);
        }
        match socket.listen(local_endpoint) {
            Ok(()) => {
                self.is_listening = true;
                Ok(0)
            }
            Err(_) => Err(SysError::EINVAL),
        }
    }

    fn shutdown(&self) -> SysResult {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);
        socket.close();
        Ok(0)
    }

    fn accept(&mut self) -> Result<(Box<dyn Socket>, Endpoint), SysError> {
        let endpoint = self.local_endpoint.ok_or(SysError::EINVAL)?;
        loop {
            let mut sockets = SOCKETS.lock();
            let socket = sockets.get::<TcpSocket>(self.handle.0);

            if socket.is_active() {
                let remote_endpoint = socket.remote_endpoint();
                drop(socket);

                let new_socket = {
                    let rx_buffer = TcpSocketBuffer::new(vec![0; TCP_RECVBUF]);
                    let tx_buffer = TcpSocketBuffer::new(vec![0; TCP_SENDBUF]);
                    let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
                    socket.listen(endpoint).unwrap();
                    let new_handle = GlobalSocketHandle(sockets.add(socket));
                    let old_handle = ::core::mem::replace(&mut self.handle, new_handle);

                    Box::new(TcpSocketState {
                        handle: old_handle,
                        local_endpoint: self.local_endpoint,
                        is_listening: false,
                    })
                };

                drop(sockets);
                poll_ifaces();
                return Ok((new_socket, Endpoint::Ip(remote_endpoint)));
            }

            drop(socket);
            SOCKET_ACTIVITY.wait(sockets);
        }
    }

    fn endpoint(&self) -> Option<Endpoint> {
        self.local_endpoint
            .clone()
            .map(|e| Endpoint::Ip(e))
            .or_else(|| {
                let mut sockets = SOCKETS.lock();
                let socket = sockets.get::<TcpSocket>(self.handle.0);
                let endpoint = socket.local_endpoint();
                if endpoint.port != 0 {
                    Some(Endpoint::Ip(endpoint))
                } else {
                    None
                }
            })
    }

    fn remote_endpoint(&self) -> Option<Endpoint> {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get::<TcpSocket>(self.handle.0);
        if socket.is_open() {
            Some(Endpoint::Ip(socket.remote_endpoint()))
        } else {
            None
        }
    }

    fn box_clone(&self) -> Box<dyn Socket> {
        Box::new(self.clone())
    }
}


