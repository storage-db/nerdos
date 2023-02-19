use crate::fs::{open_file, OpenFlags};
use crate::mm::UserBuffer;
use crate::task::*;
use alloc::string::String;
use alloc::vec::Vec;

use core::result::Result;
const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;
const FD_STDERR: usize = 2;
const CHUNK_SIZE: usize = 256;

// pub fn sys_write(fd: usize, buf: UserInPtr<u8>, len: usize) -> isize {
//     match fd {
//         FD_STDOUT | FD_STDERR => {
//             let mut count = 0;
//             while count < len {
//                 let chunk_len = CHUNK_SIZE.min(len);
//                 let chunk: [u8; CHUNK_SIZE] = unsafe { buf.add(count).read_array(chunk_len) };
//                 print!("{}", core::str::from_utf8(&chunk[..chunk_len]).unwrap());
//                 count += chunk_len;
//             }
//             count as isize
//         }
//         _ => {
//             panic!("Unsupported fd in sys_write!");
//         }
//     }
// }
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let inner = current().0.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        file.write(UserBuffer::new(translated_byte_buffer(buf, len))) as isize
    } else {
        -1
    }
}
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let inner = current().0.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        file.read(UserBuffer::new(translated_byte_buffer(buf, len))) as isize
    } else {
        -1
    }
}
/// Translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        if end - start >= 4096 {
            v.push(&mut unsafe { core::slice::from_raw_parts_mut(start as *mut u8, 4096) }[0..]);
            start += 4096;
        } else {
            v.push(
                &mut unsafe { core::slice::from_raw_parts_mut(start as *mut u8, 4096) }
                    [0..(end - start)],
            );
            start += end - start;
        }
    }
    v
}
// pub fn sys_read(fd: usize, mut buf: UserOutPtr<u8>, len: usize) -> isize {
//     match fd {
//         FD_STDIN => {
//             assert_eq!(len, 1, "Only support len = 1 in sys_read!");
//             loop {
//                 if let Some(c) = console_getchar() {
//                     buf.write(c);
//                     return 1;
//                 } else {
//                     crate::task::current().yield_now();
//                 }
//             }
//         }
//         _ => {
//             panic!("Unsupported fd in sys_read!");
//         }
//     }
// }

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current();
    // 因为没有虚拟地址映射，所以直接访问切片的指针就行。
    let path = check_and_clone_cstr(path).unwrap();
    if let Some(inode) = open_file(
        path.as_str(),
        OpenFlags::from_bits(flags).unwrap(),
    ) {
        let fd = task.inner_exclusive_access().alloc_fd();
        task.inner_exclusive_access().fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}
pub fn check_and_clone_cstr(user: *const u8) -> Result<String, String> {
    if user.is_null() {
        Ok(String::new())
    } else {
        let mut buffer = String::new();
        for i in 0.. {
            let addr = unsafe { user.add(i) };
            // let data = addr.as_ref().ok_or(String::from("EFAULT"))?;
            let data = *unsafe { (addr as *mut u8).as_mut().unwrap() };
            if data == 0 {
                break;
            }
            buffer.push(data as char);
        }
        Ok(buffer)
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current();
    if fd >= task.inner_exclusive_access().fd_table.len() {
        return -1;
    }
    if task.inner_exclusive_access().fd_table[fd].is_none() {
        return -1;
    }
    task.inner_exclusive_access().fd_table[fd].take();
    0
}
