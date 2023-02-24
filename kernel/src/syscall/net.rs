use crate::net::udp::UDP;
use crate::net::IPv4;
use crate::task::*;
use alloc::sync::Arc;

// just support udp
pub fn sys_connect(raddr: u32, lport: u16, rport: u16) -> isize {
    let process = current();
    let fd = process.inner_exclusive_access().alloc_fd();
    let udp_node = UDP::new(IPv4::from_u32(raddr), lport, rport);
    process.inner_exclusive_access().fd_table[fd] = Some(Arc::new(udp_node));
    fd as isize
}
