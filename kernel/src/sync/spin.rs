use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::instructions;

pub fn spin_lock_irqsave(lock: &AtomicBool) -> bool {
    let irq_enabled_before = !instructions::irqs_disabled();
    instructions::disable_irqs();
    while lock
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        while lock.load(Ordering::Relaxed) {
            core::hint::spin_loop();
        }
    }
    irq_enabled_before
}

pub fn spin_trylock_irqsave(lock: &AtomicBool) -> Option<bool> {
    let irq_enabled_before = !instructions::irqs_disabled();
    instructions::disable_irqs();
    if lock
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        Some(irq_enabled_before)
    } else {
        if irq_enabled_before {
            instructions::enable_irqs();
        }
        None
    }
}

pub fn spin_unlock_irqrestore(lock: &AtomicBool, irq_enabled_before: bool) {
    lock.store(false, Ordering::Release);
    if irq_enabled_before {
        instructions::enable_irqs();
    }
}

pub struct SpinNoIrqLock<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct SpinNoIrqLockGuard<'a, T: ?Sized + 'a> {
    irq_enabled_before: bool,
    lock: &'a AtomicBool,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send> Sync for SpinNoIrqLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinNoIrqLock<T> {}

impl<T> SpinNoIrqLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    pub fn lock(&self) -> SpinNoIrqLockGuard<T> {
        let irq_enabled_before = spin_lock_irqsave(&self.lock);
        SpinNoIrqLockGuard {
            irq_enabled_before,
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }

    pub fn try_lock(&self) -> Option<SpinNoIrqLockGuard<T>> {
        spin_trylock_irqsave(&self.lock).map(|irq_enabled_before| SpinNoIrqLockGuard {
            irq_enabled_before,
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        })
    }
}

impl<T: fmt::Debug> fmt::Debug for SpinNoIrqLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "SpinNoIrqLock {{ data: ")
                .and_then(|()| (*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "SpinNoIrqLock {{ <locked> }}"),
        }
    }
}

impl<T: Default> Default for SpinNoIrqLock<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for SpinNoIrqLockGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for SpinNoIrqLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for SpinNoIrqLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for SpinNoIrqLockGuard<'a, T> {
    fn drop(&mut self) {
        spin_unlock_irqrestore(self.lock, self.irq_enabled_before)
    }
}
