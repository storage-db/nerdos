use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::SchedulerTrait;
use crate::task::{current, Task};

const MAX_QUANTUM: usize = 5;

#[derive(Default)]
pub struct RRSchedulerState {
    quantum: AtomicUsize,
}

pub struct RRScheduler {
    ready_queue: VecDeque<Arc<Task>>,
}

impl RRSchedulerState {
    fn reset(&self) {
        self.quantum.store(MAX_QUANTUM, Ordering::Release);
    }

    fn decrease(&self) -> usize {
        let quantum = self.quantum.fetch_sub(1, Ordering::Release);
        assert!(quantum > 0);
        quantum - 1
    }
}

impl SchedulerTrait for RRScheduler {
    fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    fn push_ready_task_back(&mut self, t: Arc<Task>) {
        println!("1");
        t.inner_exclusive_access().sched_state().reset();
        println!("2");
        self.ready_queue.push_back(t);
    }

    fn push_ready_task_front(&mut self, t: Arc<Task>) {
        t.inner_exclusive_access().sched_state().reset();
        self.ready_queue.push_front(t);
    }

    fn pick_next_task(&mut self) -> Option<Arc<Task>> {
        self.ready_queue.pop_front()
    }

    fn timer_tick(&mut self) {
        let curr_task = current();
        if !curr_task.is_idle() && curr_task.inner_exclusive_access().sched_state().decrease() == 0 {
            curr_task.set_need_resched();
        }
    }
}
