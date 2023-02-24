use crate::arch::TrapFrame;
use crate::mm::{UserInPtr, UserOutPtr};
use crate::task::{current, spawn_task};

const MAX_STR_LEN: usize = 256;

pub fn sys_exit(exit_code: i32) -> ! {
    current().exit(exit_code);
}

pub fn sys_yield() -> isize {
    current().yield_now();
    0
}

pub fn sys_getpid() -> isize {
    current().pid().as_usize() as isize
}

pub fn sys_clone(newsp: usize, tf: &TrapFrame) -> isize {
    let new_task = current().new_clone(newsp, tf);
    let pid = new_task.pid().as_usize() as isize;
    spawn_task(new_task);
    pid
}

pub fn sys_fork(tf: &TrapFrame) -> isize {
    let new_task = current().new_fork(tf);
    let pid = new_task.pid().as_usize() as isize;
    spawn_task(new_task);
    pid
}

pub fn sys_exec(path: UserInPtr<u8>, tf: &mut TrapFrame) -> isize {
    let (path_buf, len) = path.read_str::<MAX_STR_LEN>();
    let path = core::str::from_utf8(&path_buf[..len]).unwrap();
    current().exec(path, tf)
}

pub fn sys_waitpid(pid: isize, mut exit_code_ptr: UserOutPtr<i32>, options: u32) -> isize {
    if let Some((pid, exit_code)) = current().waitpid(pid, options) {
        if !exit_code_ptr.is_null() {
            exit_code_ptr.write(exit_code);
        }
        pid.as_usize() as _
    } else {
        -1
    }
}
