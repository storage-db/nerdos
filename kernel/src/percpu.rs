use alloc::sync::Arc;

use crate::arch::{instructions, ArchPerCpu};
use crate::config::MAX_CPUS;
use crate::sync::{LazyInit, PerCpuData};
use crate::task::{CurrentTask, Task};

static CPUS: [LazyInit<PerCpu>; MAX_CPUS] = [LazyInit::new(); MAX_CPUS];

/// Each CPU can only accesses its own `PerCpu` instance.
#[repr(C)]
pub struct PerCpu {
    self_vaddr: usize,
    id: usize,
    idle_task: Arc<Task>,
    current_task: PerCpuData<Arc<Task>>,
    arch: PerCpuData<ArchPerCpu>,
}

impl PerCpu {
    fn new(id: usize) -> Self {
        let idle_task = Task::new_idle();
        Self {
            self_vaddr: &CPUS[id] as *const _ as usize,
            id,
            current_task: PerCpuData::new(idle_task.clone()),
            idle_task,
            arch: PerCpuData::new(ArchPerCpu::new()),
        }
    }

    fn current<'a>() -> &'a Self {
        Self::current_mut()
    }

    fn current_mut<'a>() -> &'a mut Self {
        unsafe { &mut *(instructions::thread_pointer() as *mut Self) }
    }

    pub fn current_cpu_id() -> usize {
        Self::current().id
    }

    pub fn idle_task<'a>() -> &'a Arc<Task> {
        &Self::current().idle_task
    }

    pub fn current_task<'a>() -> CurrentTask<'a> {
        // Safety: Even if there is an interrupt and task preemption after
        // calling this method, the reference of percpu data (e.g., `current_task`) can keep unchanged
        // since it will be restored after context switches.
        CurrentTask(unsafe { Self::current().current_task.as_ref() })
    }

    pub unsafe fn set_current_task(task: Arc<Task>) {
        // We must disable interrupts and task preemption when update this field.
        assert!(instructions::irqs_disabled());
        let old_task = core::mem::replace(Self::current().current_task.as_mut(), task);
        drop(old_task)
    }

    #[allow(dead_code)]
    pub fn current_arch_data<'a>() -> &'a PerCpuData<ArchPerCpu> {
        &Self::current().arch
    }
}

#[allow(dead_code)]
pub const PERCPU_ARCH_OFFSET: usize = memoffset::offset_of!(PerCpu, arch);

pub fn init_percpu_early() {
    let cpu_id = 0;
    CPUS[cpu_id].init_by(PerCpu::new(cpu_id));
    unsafe {
        instructions::set_thread_pointer(CPUS[cpu_id].self_vaddr);
        PerCpu::current_arch_data().as_mut().init(cpu_id);
    }
}

pub fn init_percpu() {
    let idle_task = unsafe { Arc::get_mut_unchecked(&mut PerCpu::current_mut().idle_task) };
    idle_task.init_idle();
}
