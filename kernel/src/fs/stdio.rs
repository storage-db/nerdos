//!Stdin & Stdout
use super::File;
use crate::drivers::uart::console_getchar;
use crate::mm::UserBuffer;
use crate::task::current;
///Standard input
pub struct Stdin;
///Standard output
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        loop {
            if let Some(c) = console_getchar() {
                if c == 0 {
                    continue;
                } else {
                    unsafe {
                        user_buf.buffers[0].as_mut_ptr().write_volatile(c);
                    }
                    break;
                }
            } else {
                current().yield_now();
            }
        }
        1
    }
    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
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
impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }
}
