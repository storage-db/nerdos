use alloc::sync::Arc;
use core::cell::UnsafeCell;

use super::schedule::{Scheduler, SchedulerTrait};
use super::structs::{CurrentTask, Task, TaskState, ROOT_TASK};
use crate::percpu::PerCpu;
use crate::sync::{LazyInit, SpinNoIrqLock};
use crate::timer::{current_time, TimeValue};

pub struct TaskManager {
    scheduler: Scheduler,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
        }
    }

    pub fn scheduler_timer_tick(&mut self) {
        self.scheduler.timer_tick();
    }

    pub fn spawn(&mut self, t: Arc<Task>) {
        assert!(t.state() == TaskState::Ready);
        self.scheduler.push_ready_task_back(t);
    }

    fn switch_to(&self, curr_task: &Arc<Task>, next_task: Arc<Task>) {
        trace!(
            "context switch: {:?} -> {:?}",
            curr_task.pid(),
            next_task.pid()
        );
        next_task.set_state(TaskState::Running);
        if Arc::ptr_eq(curr_task, &next_task) {
            return;
        }

        let curr_ctx_ptr = curr_task.context().as_ptr();
        let next_ctx_ptr = next_task.context().as_ptr();

        // Decrement the strong reference count of `curr_task` and `next_task`,
        // but won't drop them until `waitpid()` is called,
        assert!(Arc::strong_count(curr_task) > 1);
        assert!(Arc::strong_count(&next_task) > 1);

        unsafe {
            PerCpu::set_current_task(next_task);
            (*curr_ctx_ptr).switch_to(&*next_ctx_ptr);
        }
    }

    fn resched(&mut self, curr_task: &CurrentTask) {
        assert!(curr_task.state() != TaskState::Running);
        if let Some(next_task) = self.scheduler.pick_next_task() {
            // let `next_task` hold its ownership to avoid clone
            self.switch_to(curr_task, next_task);
        } else {
            self.switch_to(curr_task, PerCpu::idle_task().clone());
        }
    }

    pub fn yield_current(&mut self, curr_task: &CurrentTask) {
        assert!(curr_task.state() == TaskState::Running);
        curr_task.set_state(TaskState::Ready);
        if !curr_task.is_idle() {
            self.scheduler.push_ready_task_back(curr_task.clone_task());
        }
        self.resched(curr_task);
    }

    pub fn unblock_task(&mut self, task: Arc<Task>) -> bool {
        if task.state() == TaskState::Sleeping {
            task.set_state(TaskState::Ready);
            self.scheduler.push_ready_task_front(task);
            true
        } else {
            false
        }
    }

    pub fn block_current(&mut self, curr_task: &CurrentTask) {
        // assert not in spin lock
        assert!(curr_task.state() == TaskState::Running);
        assert!(!curr_task.is_idle());
        curr_task.set_state(TaskState::Sleeping);
        self.resched(curr_task);
    }

    pub fn sleep_current(&mut self, curr_task: &CurrentTask, deadline: TimeValue) {
        assert!(curr_task.state() == TaskState::Running);
        assert!(!curr_task.is_idle());
        if current_time() < deadline {
            let curr_task_clone = curr_task.clone_task();
            crate::timer::set_timer(deadline, move |_| {
                TASK_MANAGER.lock().unblock_task(curr_task_clone);
                CurrentTask::get().set_need_resched();
            });
            self.block_current(curr_task);
        }
    }

    pub fn exit_current(&mut self, curr_task: &CurrentTask, exit_code: i32) -> ! {
        assert!(!curr_task.is_idle());
        assert!(!curr_task.is_root());
        assert!(curr_task.state() == TaskState::Running);

        // Make all child tasks as the children of the root task
        {
            let mut notify = false;
            let mut children = curr_task.children.lock();
            for c in children.iter() {
                ROOT_TASK.add_child(c);
                if c.state() == TaskState::Zombie {
                    notify = true;
                }
            }
            children.clear();
            if notify {
                ROOT_TASK.wait_children_exit.notify_locked(self);
            }
        }

        curr_task.set_state(TaskState::Zombie);
        curr_task.set_exit_code(exit_code);

        curr_task
            .parent
            .lock()
            .upgrade()
            .unwrap()
            .wait_children_exit
            .notify_locked(self);

        self.resched(curr_task);
        unreachable!("task exited!");
    }

    #[allow(dead_code)]
    pub fn dump_all_tasks(&self) {
        if ROOT_TASK.children.lock().len() == 0 {
            return;
        }
        println!(
            "{:>4} {:>4} {:>6} {:>4}  STATE",
            "PID", "PPID", "#CHILD", "#REF",
        );
        ROOT_TASK.traverse(&|t: &Arc<Task>| {
            let pid = t.pid().as_usize();
            let ref_count = Arc::strong_count(t);
            let children_count = t.children.lock().len();
            let state = t.state();
            let shared = if t.is_shared_with_parent() { 'S' } else { ' ' };
            if let Some(p) = t.parent.lock().upgrade() {
                let ppid = p.pid().as_usize();
                println!(
                    "{:>4}{}{:>4} {:>6} {:>4}  {:?}",
                    pid, shared, ppid, children_count, ref_count, state
                );
            } else {
                println!(
                    "{:>4} {:>4} {:>6} {:>4}  {:?}",
                    pid, '-', children_count, ref_count, state
                );
            }
        });
    }
}

/// A wrapper structure which can only be accessed while holding the lock of `TASK_MANAGER`.
pub struct TaskLockedCell<T> {
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for TaskLockedCell<T> {}

impl<T> TaskLockedCell<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }

    pub fn as_ptr(&self) -> *mut T {
        assert!(TASK_MANAGER.is_locked());
        assert!(crate::arch::instructions::irqs_disabled());
        self.data.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

pub(super) static TASK_MANAGER: LazyInit<SpinNoIrqLock<TaskManager>> = LazyInit::new();

pub(super) fn init() {
    TASK_MANAGER.init_by(SpinNoIrqLock::new(TaskManager::new()));
}
