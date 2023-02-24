use alloc::sync::Arc;

use super::manager::{TaskManager, TASK_MANAGER};
use super::{current, Task};
use crate::sync::SpinNoIrqLock;

pub struct WaitCurrent {
    current: SpinNoIrqLock<Option<Arc<Task>>>,
}

impl WaitCurrent {
    pub const fn new() -> Self {
        Self {
            current: SpinNoIrqLock::new(None),
        }
    }

    pub fn wait(&self) {
        assert!(!TASK_MANAGER.is_locked());
        let mut m = TASK_MANAGER.lock();
        let curr_task = current();
        assert!(self
            .current
            .lock()
            .replace(curr_task.clone_task())
            .is_none());
        m.block_current(&curr_task);
    }

    #[allow(dead_code)]
    pub fn notify(&self) -> bool {
        assert!(!TASK_MANAGER.is_locked());
        self.notify_locked(&mut TASK_MANAGER.lock())
    }

    pub(super) fn notify_locked(&self, m: &mut TaskManager) -> bool {
        assert!(TASK_MANAGER.is_locked());
        if let Some(t) = self.current.lock().take() {
            m.unblock_task(t)
        } else {
            false
        }
    }
}
