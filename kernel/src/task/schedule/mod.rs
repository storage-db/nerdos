#![allow(unused_imports)]

mod round_robin;

use crate::task::Task;
use alloc::sync::Arc;

use round_robin::{RRScheduler, RRSchedulerState};

pub trait SchedulerTrait {
    fn new() -> Self;
    fn push_ready_task_front(&mut self, t: Arc<Task>);
    fn push_ready_task_back(&mut self, t: Arc<Task>);
    fn pick_next_task(&mut self) -> Option<Arc<Task>>;
    fn timer_tick(&mut self);
}

pub type SchedulerState = RRSchedulerState;
pub type Scheduler = RRScheduler;
