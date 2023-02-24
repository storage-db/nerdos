#![allow(dead_code)]
#![allow(clippy::uninit_assumed_init)]

use core::mem::{align_of, size_of, MaybeUninit};
use core::{fmt, marker::PhantomData};

use crate::config::{USER_ASPACE_BASE, USER_ASPACE_SIZE};

#[allow(clippy::absurd_extreme_comparisons)]
const fn uaccess_ok(vaddr: usize, size: usize) -> bool {
    vaddr != 0 && USER_ASPACE_BASE <= vaddr && vaddr - USER_ASPACE_BASE <= USER_ASPACE_SIZE - size
}

unsafe fn copy_from_user<T>(kdst: *mut T, usrc: *const T, len: usize) {
    assert!(uaccess_ok(usrc as usize, len * size_of::<T>()));
    kdst.copy_from_nonoverlapping(usrc, len);
}

unsafe fn copy_to_user<T>(udst: *mut T, ksrc: *const T, len: usize) {
    assert!(uaccess_ok(udst as usize, len * size_of::<T>()));
    udst.copy_from_nonoverlapping(ksrc, len);
}

unsafe fn copy_from_user_str(kdst: *mut u8, usrc: *const u8, max_len: usize) -> usize {
    assert!(uaccess_ok(usrc as usize, 1));
    let mut len = 0;
    let mut kdst = kdst;
    let mut usrc = usrc;
    while len < max_len {
        assert!((usrc as usize) < USER_ASPACE_BASE + USER_ASPACE_SIZE);
        let c = usrc.read();
        if c == b'\0' {
            break;
        }
        kdst.write(c);
        len += 1;
        kdst = kdst.add(1);
        usrc = usrc.add(1);
    }
    kdst.write(b'\0');
    len
}

pub trait Policy {}
pub trait ReadPolicy: Policy {}
pub trait WritePolicy: Policy {}
pub enum In {}
pub enum Out {}
pub enum InOut {}

impl Policy for In {}
impl ReadPolicy for In {}
impl Policy for Out {}
impl WritePolicy for Out {}
impl Policy for InOut {}
impl ReadPolicy for InOut {}
impl WritePolicy for InOut {}

pub type UserInPtr<T> = UserPtr<T, In>;
pub type UserOutPtr<T> = UserPtr<T, Out>;
pub type UserInOutPtr<T> = UserPtr<T, InOut>;

pub struct UserPtr<T, P: Policy> {
    ptr: *mut T,
    _phantom: PhantomData<P>,
}

impl<T, P: Policy> fmt::Debug for UserPtr<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UserPtr({:?})", self.ptr)
    }
}

impl<T, P: Policy> From<usize> for UserPtr<T, P> {
    fn from(user_vadddr: usize) -> Self {
        assert!(user_vadddr % align_of::<T>() == 0);
        Self {
            ptr: user_vadddr as *mut T,
            _phantom: PhantomData,
        }
    }
}

impl<T, P: Policy> UserPtr<T, P> {
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }

    pub unsafe fn add(&self, count: usize) -> Self {
        Self {
            ptr: self.ptr.add(count),
            _phantom: PhantomData,
        }
    }
}

impl<T, P: ReadPolicy> UserPtr<T, P> {
    pub fn read(&self) -> T {
        let mut value = MaybeUninit::uninit();
        unsafe {
            copy_from_user(value.as_mut_ptr(), self.ptr, 1);
            value.assume_init()
        }
    }

    pub fn read_array<const N: usize>(&self, max_len: usize) -> [T; N] {
        let mut buf: [T; N] = unsafe { MaybeUninit::uninit().assume_init() };
        unsafe { copy_from_user(buf.as_mut_ptr(), self.ptr, max_len.min(N)) };
        buf
    }
}

impl<P: ReadPolicy> UserPtr<u8, P> {
    pub fn read_str<const N: usize>(&self) -> ([u8; N], usize) {
        let mut buf: [u8; N] = unsafe { MaybeUninit::uninit().assume_init() };
        let len = unsafe { copy_from_user_str(buf.as_mut_ptr(), self.ptr, N - 1) };
        (buf, len)
    }
}

impl<T, P: WritePolicy> UserPtr<T, P> {
    pub fn write(&mut self, value: T) {
        unsafe { copy_to_user(self.ptr, &value as *const T, 1) }
    }

    pub fn write_buf(&mut self, buf: &[T]) {
        unsafe { copy_to_user(self.ptr, buf.as_ptr(), buf.len()) };
    }
}
