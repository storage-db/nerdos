use core::sync::atomic::{AtomicUsize, Ordering};

pub type IrqHandler = fn();

pub struct IrqHandlerTable<const IRQ_COUNT: usize> {
    handlers: [AtomicUsize; IRQ_COUNT],
}

impl<const IRQ_COUNT: usize> IrqHandlerTable<IRQ_COUNT> {
    #[allow(clippy::declare_interior_mutable_const)]
    pub const fn new() -> Self {
        const EMPTY: AtomicUsize = AtomicUsize::new(0);
        Self {
            handlers: [EMPTY; IRQ_COUNT],
        }
    }

    pub fn register_handler(&self, vector: usize, handler: IrqHandler) {
        self.handlers[vector].store(handler as usize, Ordering::Release);
    }

    pub fn handle(&self, vector: usize) {
        let handler = self.handlers[vector].load(Ordering::Acquire);
        if handler != 0 {
            trace!("IRQ {}", vector);
            let handler: IrqHandler = unsafe { core::mem::transmute(handler) };
            handler();
        } else {
            warn!("Unhandled IRQ {}", vector);
        }
    }
}
