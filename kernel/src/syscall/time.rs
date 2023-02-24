use crate::mm::{UserInPtr, UserOutPtr};
use crate::timer::{current_time, TimeValue};

const TIMER_ABSTIME: u32 = 1;

#[repr(C)]
pub struct TimeSpec {
    /// seconds
    pub sec: usize,
    /// nano seconds
    pub nsec: usize,
}

impl From<TimeSpec> for TimeValue {
    fn from(ts: TimeSpec) -> Self {
        Self::new(ts.sec as _, ts.nsec as _)
    }
}

impl From<TimeValue> for TimeSpec {
    fn from(tv: TimeValue) -> Self {
        Self {
            sec: tv.as_secs() as _,
            nsec: tv.subsec_nanos() as _,
        }
    }
}

pub fn sys_get_time_ms() -> isize {
    current_time().as_millis() as isize
}

pub fn sys_clock_gettime(_clock_id: u32, mut ts: UserOutPtr<TimeSpec>) -> isize {
    ts.write(TimeSpec::from(current_time()));
    0
}

pub fn sys_clock_nanosleep(_clock_id: u32, flags: u32, req: UserInPtr<TimeSpec>) -> isize {
    let deadline = if (flags & TIMER_ABSTIME) != 0 {
        req.read().into()
    } else {
        current_time() + req.read().into()
    };
    crate::task::current().sleep(deadline);
    0
}
