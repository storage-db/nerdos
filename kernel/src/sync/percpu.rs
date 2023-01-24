use core::cell::UnsafeCell;

/// A wrapper contains the data owned by each CPU and can only be accessed by
/// that CPU.
#[repr(transparent)]
pub struct PerCpuData<T> {
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for PerCpuData<T> {}

impl<T> PerCpuData<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }

    /// # Safety
    /// The user must consider whether an interrupt occurs after calling this
    /// function and cause the value to be modified.
    pub unsafe fn as_ref(&self) -> &T {
        &*self.data.get()
    }

    /// # Safety
    /// The user must consider whether an interrupt occurs after calling this
    /// function and cause the value to be modified.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut(&self) -> &mut T {
        &mut *self.data.get()
    }
}
