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


