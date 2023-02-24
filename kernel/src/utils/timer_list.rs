use alloc::{boxed::Box, collections::BinaryHeap};
use core::cmp::{Ord, Ordering, PartialOrd};
use core::time::Duration;

pub type TimeValue = Duration;

type TimerCallback = Box<dyn FnOnce(TimeValue) + Send + Sync + 'static>;

struct TimerEvent {
    deadline: TimeValue,
    callback: TimerCallback,
}

pub struct TimerList {
    events: BinaryHeap<TimerEvent>,
}

impl PartialOrd for TimerEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.deadline.partial_cmp(&self.deadline) // reverse ordering for Min-heap
    }
}

impl Ord for TimerEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.deadline.cmp(&self.deadline) // reverse ordering for Min-heap
    }
}

impl PartialEq for TimerEvent {
    fn eq(&self, other: &Self) -> bool {
        self.deadline.eq(&other.deadline)
    }
}

impl Eq for TimerEvent {}

impl TimerList {
    pub fn new() -> Self {
        Self {
            events: BinaryHeap::new(),
        }
    }

    pub fn set(
        &mut self,
        deadline: TimeValue,
        callback: impl FnOnce(TimeValue) + Send + Sync + 'static,
    ) {
        self.events.push(TimerEvent {
            deadline,
            callback: Box::new(callback),
        });
    }

    pub fn next_deadline(&self) -> Option<TimeValue> {
        self.events.peek().map(|e| e.deadline)
    }

    pub fn expire_one(&mut self, now: TimeValue) -> Option<TimeValue> {
        if let Some(e) = self.events.peek() {
            if e.deadline <= now {
                let e = self.events.pop().unwrap();
                (e.callback)(now);
                return Some(e.deadline);
            }
        }
        None
    }
}
