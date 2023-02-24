use super::super::misc::sbi;

pub fn init_early() {}
pub fn init() {}

pub fn console_putchar(c: u8) {
    sbi::console_putchar(c as usize)
}

pub fn console_getchar() -> Option<u8> {
    match sbi::console_getchar() {
        -1 => None,
        c => Some(c as u8),
    }
}
