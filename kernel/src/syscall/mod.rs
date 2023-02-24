const SYSCALL_READ: usize = 0;
const SYSCALL_WRITE: usize = 1;
const SYSCALL_OPEN: usize = 2;
const SYSCALL_CLOSE: usize = 3;
const SYSCALL_YIELD: usize = 24;
pub const SYSCALL_CONNECT: usize = 29;
const SYSCALL_GETPID: usize = 39;
const SYSCALL_CLONE: usize = 56;
const SYSCALL_FORK: usize = 57;
const SYSCALL_EXEC: usize = 59;
const SYSCALL_EXIT: usize = 60;
const SYSCALL_WAITPID: usize = 61;
const SYSCALL_GET_TIME_MS: usize = 96;
const SYSCALL_CLOCK_GETTIME: usize = 228;
const SYSCALL_CLOCK_NANOSLEEP: usize = 230;

mod fs;
mod task;
mod time;
mod net;
use self::fs::*;
use self::task::*;
use self::time::*;
use self::net::*;
use crate::arch::{instructions, TrapFrame};
pub fn syscall(
    tf: &mut TrapFrame,
    syscall_id: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> isize {
    instructions::enable_irqs();
    debug!(
        "syscall {} enter <= ({:#x}, {:#x}, {:#x})",
        syscall_id, arg0, arg1, arg2
    );
    let ret = match syscall_id {
        SYSCALL_READ => sys_read(arg0, arg1 as *const u8, arg2),
        SYSCALL_WRITE => sys_write(arg0, arg1 as *const u8, arg2),
        SYSCALL_OPEN => sys_open(arg0 as *const u8, arg1 as u32),
        SYSCALL_CLOSE => sys_close(arg0),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_CLONE => sys_clone(arg0, tf),
        SYSCALL_FORK => sys_fork(tf),
        SYSCALL_EXEC => sys_exec(arg0.into(), tf),
        SYSCALL_EXIT => sys_exit(arg0 as i32),
        SYSCALL_WAITPID => sys_waitpid(arg0 as _, arg1.into(), arg2 as _),
        SYSCALL_GET_TIME_MS => sys_get_time_ms(),
        SYSCALL_CLOCK_GETTIME => sys_clock_gettime(arg0 as _, arg1.into()),
        SYSCALL_CLOCK_NANOSLEEP => sys_clock_nanosleep(arg0 as _, arg1 as _, arg2.into()),
        // socket
        SYSCALL_CONNECT => sys_connect(arg0 as _, arg1 as _, arg2 as _),
        _ => {
            warn!("Unsupported syscall_id: {}", syscall_id);
            crate::task::current().exit(-1);
        }
    };
    debug!("syscall {} ret => {:#x}", syscall_id, ret);
    instructions::disable_irqs();
    ret
}
