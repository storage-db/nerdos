use riscv::register::sie;

use crate::sync::LazyInit;
use crate::utils::irq_handler::IrqHandler;

const INT_BASE: usize = 1 << (usize::BITS - 1);
const S_SOFT: usize = INT_BASE + 1;
const S_TIMER: usize = INT_BASE + 5;
const S_EXT: usize = INT_BASE + 9;

static SOFT_HANDLER: LazyInit<IrqHandler> = LazyInit::new();
static TIMER_HANDLER: LazyInit<IrqHandler> = LazyInit::new();
static EXT_HANDLER: LazyInit<IrqHandler> = LazyInit::new();

#[repr(usize)]
#[allow(dead_code)]
#[allow(clippy::enum_clike_unportable_variant)]
pub enum ScauseIntCode {
    Soft = S_SOFT,
    Timer = S_TIMER,
    External = S_EXT,
}

fn with_cause<F1, F2, F3, T>(cause: usize, soft_op: F1, timer_op: F2, ext_op: F3) -> T
where
    F1: FnOnce() -> T,
    F2: FnOnce() -> T,
    F3: FnOnce() -> T,
{
    match cause {
        S_SOFT => soft_op(),
        S_TIMER => timer_op(),
        S_EXT => ext_op(),
        _ => panic!("invalid trap cause: {:#x}", cause),
    }
}

pub fn handle_irq(cause: usize) {
    let handler = with_cause(cause, || &SOFT_HANDLER, || &TIMER_HANDLER, || &EXT_HANDLER);
    if handler.is_init() {
        trace!("Trap cause {:#x}", cause);
        handler();
    } else {
        warn!("Unhandled trap cause: {:#x}", cause);
    }
}

pub fn register_handler(cause: usize, handler: IrqHandler) {
    with_cause(
        cause,
        || SOFT_HANDLER.init_by(handler),
        || TIMER_HANDLER.init_by(handler),
        || EXT_HANDLER.init_by(handler),
    );
}

pub fn set_enable(cause: usize, enable: bool) {
    unsafe {
        if enable {
            with_cause(
                cause,
                || sie::set_ssoft(),
                || sie::set_stimer(),
                || sie::set_sext(),
            );
        } else {
            with_cause(
                cause,
                || sie::clear_ssoft(),
                || sie::clear_stimer(),
                || sie::clear_sext(),
            );
        }
    }
}

pub fn init() {}
