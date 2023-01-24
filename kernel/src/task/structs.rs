use alloc::sync::{Arc, Weak};
use alloc::{boxed::Box, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicUsize, Ordering};

use super::manager::{TaskLockedCell, TASK_MANAGER};
use super::schedule::SchedulerState;
use super::wait_queue::WaitCurrent;
use crate::arch::{instructions, TaskContext, TrapFrame};
use crate::config::KERNEL_STACK_SIZE;
use crate::loader;
use crate::mm::{kernel_aspace, MemorySet, VirtAddr};
use crate::percpu::PerCpu;
use crate::sync::{LazyInit, Mutex};
use crate::timer::TimeValue;

pub(super) static ROOT_TASK: LazyInit<Arc<Task>> = LazyInit::new();

#[derive(Debug)]
enum EntryState {
    Kernel { pc: usize, arg: usize },
    User(Box<TrapFrame>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TaskId(usize);

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TaskState {
    Ready = 1,
    Running = 2,
    Sleeping = 3,
    Zombie = 4,
}

pub struct Task {
    id: TaskId,
    is_kernel: bool,
    is_shared: bool,
    entry: EntryState,

    state: AtomicU8,
    exit_code: AtomicI32,
    need_resched: AtomicBool,
    sched_state: SchedulerState,

    kstack: Stack<KERNEL_STACK_SIZE>,
    ctx: TaskLockedCell<TaskContext>,

    pub(super) wait_children_exit: WaitCurrent,

    vm: Option<Arc<Mutex<MemorySet>>>,
    pub(super) parent: Mutex<Weak<Task>>,
    pub(super) children: Mutex<Vec<Arc<Task>>>,
}

impl TaskId {
    const IDLE_TASK_ID: Self = Self(0);

    fn alloc() -> Self {
        static NEXT_PID: AtomicUsize = AtomicUsize::new(1);
        Self(NEXT_PID.fetch_add(1, Ordering::AcqRel))
    }

    pub const fn as_usize(&self) -> usize {
        self.0
    }
}

impl From<usize> for TaskId {
    fn from(pid: usize) -> Self {
        Self(pid)
    }
}

impl From<u8> for TaskState {
    fn from(state: u8) -> Self {
        match state {
            1 => Self::Ready,
            2 => Self::Running,
            3 => Self::Sleeping,
            4 => Self::Zombie,
            _ => panic!("invalid task state: {}", state),
        }
    }
}

impl Task {
    fn new_common(id: TaskId) -> Self {
        Self {
            id,
            is_kernel: false,
            is_shared: false,
            entry: EntryState::Kernel { pc: 0, arg: 0 },

            state: AtomicU8::new(TaskState::Ready as u8),
            exit_code: AtomicI32::new(0),
            need_resched: AtomicBool::new(false),
            sched_state: SchedulerState::default(),

            kstack: Stack::default(),
            ctx: TaskLockedCell::new(TaskContext::default()),

            wait_children_exit: WaitCurrent::new(),

            vm: None,
            parent: Mutex::new(Weak::default()),
            children: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn add_child(self: &Arc<Self>, child: &Arc<Task>) {
        *child.parent.lock() = Arc::downgrade(self);
        self.children.lock().push(child.clone());
    }

    pub fn new_idle() -> Arc<Self> {
        let mut t = Self::new_common(TaskId::IDLE_TASK_ID);
        t.is_kernel = true;
        Arc::new(t)
    }

    pub fn init_idle(&mut self) {
        self.set_state(TaskState::Running);
        self.ctx.get_mut().init(
            0,
            self.kstack.top(),
            kernel_aspace().page_table_root(),
            true,
        );
    }

    pub fn new_kernel(entry: fn(usize) -> usize, arg: usize) -> Arc<Self> {
        let mut t = Self::new_common(TaskId::alloc());
        t.is_kernel = true;
        t.entry = EntryState::Kernel {
            pc: entry as usize,
            arg,
        };
        t.ctx.get_mut().init(
            task_entry as _,
            t.kstack.top(),
            kernel_aspace().page_table_root(),
            true,
        );

        let t = Arc::new(t);
        if !t.is_root() {
            ROOT_TASK.add_child(&t);
        }
        t
    }

    pub fn new_user(path: &str) -> Arc<Self> {
        let elf_data = loader::get_app_data_by_name(path).expect("new_user: no such app");
        let mut vm = MemorySet::new();
        let (entry, ustack_top) = vm.load_user(elf_data);

        let mut t = Self::new_common(TaskId::alloc());
        t.entry = EntryState::User(Box::new(TrapFrame::new_user(entry, ustack_top, 0)));
        t.ctx
            .get_mut()
            .init(task_entry as _, t.kstack.top(), vm.page_table_root(), false);
        t.vm = Some(Arc::new(Mutex::new(vm)));

        let t = Arc::new(t);
        ROOT_TASK.add_child(&t);
        t
    }

    pub fn new_clone(self: &Arc<Self>, newsp: usize, tf: &TrapFrame) -> Arc<Self> {
        assert!(!self.is_kernel_task());
        let mut t = Self::new_common(TaskId::alloc());
        t.is_shared = true;
        let vm = self.vm.as_ref().unwrap().clone();
        t.entry = EntryState::User(Box::new(tf.new_clone(VirtAddr::new(newsp))));
        t.ctx.get_mut().init(
            task_entry as _,
            t.kstack.top(),
            vm.lock().page_table_root(),
            false,
        );
        t.vm = Some(vm);

        let t = Arc::new(t);
        self.add_child(&t);
        t
    }

    pub fn new_fork(self: &Arc<Self>, tf: &TrapFrame) -> Arc<Self> {
        assert!(!self.is_kernel_task());
        let mut t = Self::new_common(TaskId::alloc());
        let vm = self.vm.as_ref().unwrap().lock().dup();
        t.entry = EntryState::User(Box::new(tf.new_fork()));
        t.ctx
            .get_mut()
            .init(task_entry as _, t.kstack.top(), vm.page_table_root(), false);
        t.vm = Some(Arc::new(Mutex::new(vm)));

        let t = Arc::new(t);
        self.add_child(&t);
        t
    }

    pub const fn pid(&self) -> TaskId {
        self.id
    }

    pub const fn is_kernel_task(&self) -> bool {
        self.is_kernel
    }

    pub const fn is_root(&self) -> bool {
        self.id.as_usize() == 1
    }

    pub const fn is_idle(&self) -> bool {
        self.id.as_usize() == 0
    }

    pub const fn is_shared_with_parent(&self) -> bool {
        self.is_shared
    }

    pub fn state(&self) -> TaskState {
        self.state.load(Ordering::SeqCst).into()
    }

    pub(super) fn set_state(&self, state: TaskState) {
        self.state.store(state as u8, Ordering::SeqCst)
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code.load(Ordering::SeqCst)
    }

    pub(super) fn set_exit_code(&self, exit_code: i32) {
        self.exit_code.store(exit_code, Ordering::SeqCst)
    }

    pub fn need_resched(&self) -> bool {
        self.need_resched.load(Ordering::SeqCst)
    }

    pub(super) const fn context(&self) -> &TaskLockedCell<TaskContext> {
        &self.ctx
    }

    pub(super) const fn sched_state(&self) -> &SchedulerState {
        &self.sched_state
    }

    pub(super) fn traverse(self: &Arc<Self>, func: &impl Fn(&Arc<Task>)) {
        func(self);
        for c in self.children.lock().iter() {
            c.traverse(func);
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        debug!("Task({}) dropped", self.pid().as_usize());
    }
}

fn task_entry() -> ! {
    // release the lock that was implicitly held across the reschedule
    unsafe { TASK_MANAGER.force_unlock() };
    instructions::enable_irqs();
    let task = CurrentTask::get();
    match &task.entry {
        EntryState::Kernel { pc, arg } => {
            let entry: fn(usize) -> i32 = unsafe { core::mem::transmute(*pc) };
            let ret = entry(*arg);
            task.exit(ret as _);
        }
        EntryState::User(tf) => {
            unsafe { tf.exec(task.kstack.top()) };
        }
    }
}

pub struct CurrentTask<'a>(pub &'a Arc<Task>);

impl<'a> CurrentTask<'a> {
    pub(super) fn get() -> Self {
        PerCpu::current_task()
    }

    pub fn clone_task(&self) -> Arc<Task> {
        self.0.clone()
    }

    pub fn clear_need_resched(&self) {
        self.0.need_resched.store(false, Ordering::SeqCst);
    }

    pub fn set_need_resched(&self) {
        self.0.need_resched.store(true, Ordering::SeqCst);
    }

    pub fn yield_now(&self) {
        TASK_MANAGER.lock().yield_current(self);
    }

    pub fn sleep(&self, deadline: TimeValue) {
        TASK_MANAGER.lock().sleep_current(self, deadline);
    }

    pub fn exit(&self, exit_code: i32) -> ! {
        info!("task exit with code {}", exit_code);
        if let Some(vm) = self.vm.as_ref() {
            if Arc::strong_count(vm) == 1 {
                vm.lock().clear(); // drop memory set before lock
            }
        }
        TASK_MANAGER.lock().exit_current(self, exit_code)
    }

    pub fn exec(&self, path: &str, tf: &mut TrapFrame) -> isize {
        assert!(!self.is_kernel_task());
        assert_eq!(Arc::strong_count(self.vm.as_ref().unwrap()), 1);
        if let Some(elf_data) = loader::get_app_data_by_name(path) {
            let mut vm = self.vm.as_ref().unwrap().lock();
            vm.clear();
            let (entry, ustack_top) = vm.load_user(elf_data);
            *tf = TrapFrame::new_user(entry, ustack_top, 0);
            instructions::flush_tlb_all();
            0
        } else {
            -1
        }
    }

    pub fn waitpid(&self, pid: isize, _options: u32) -> Option<(TaskId, i32)> {
        let mut found_pid = false;
        for t in self.children.lock().iter() {
            if pid == -1 || t.pid().as_usize() == pid as usize {
                found_pid = true;
                break;
            }
        }
        if !found_pid {
            return None;
        }

        loop {
            {
                let mut children = self.children.lock();
                for (idx, t) in children.iter().enumerate() {
                    if (pid == -1 || t.pid().as_usize() == pid as usize)
                        && t.state() == TaskState::Zombie
                    {
                        let child = children.remove(idx);
                        assert_eq!(Arc::strong_count(&child), 1);
                        return Some((child.pid(), child.exit_code()));
                    }
                }
            }
            self.wait_children_exit.wait();
        }
    }
}

impl<'a> core::ops::Deref for CurrentTask<'a> {
    type Target = Arc<Task>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

struct Stack<const N: usize>(Box<[u8]>);

impl<const N: usize> Stack<N> {
    pub fn default() -> Self {
        Self(Box::from(alloc::vec![0; N]))
    }

    pub fn top(&self) -> VirtAddr {
        VirtAddr::new(self.0.as_ptr_range().end as usize)
    }
}
