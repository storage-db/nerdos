mod manager;
mod schedule;
mod structs;
mod wait_queue;

pub use structs::{CurrentTask, Task, TaskId};

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

pub use self::manager::TASK_MANAGER;
use self::structs::ROOT_TASK;
use crate::arch::instructions;

static TASK_INITED: AtomicBool = AtomicBool::new(false);

pub fn is_init() -> bool {
    TASK_INITED.load(Ordering::SeqCst)
}

pub fn init() {
    println!("Initializing task manager...");
    manager::init();

    ROOT_TASK.init_by(Task::new_kernel(
        |_| loop {
            let curr_task = current();
            while curr_task.waitpid(-1, 0).is_some() {}
            // instructions::wait_for_ints();
            info!("No more tasks to run, shutdown!");
            crate::drivers::misc::shutdown();
        },
        0,
    ));

    let test_kernel_task = |arg: usize| {
        println!(
            "test kernel task: pid = {:?}, arg = {:#x}",
            current().pid(),
            arg
        );
        0
    };

    let mut m = TASK_MANAGER.lock();
    m.spawn(ROOT_TASK.clone());
    m.spawn(Task::new_kernel(test_kernel_task, 0xdead));
    m.spawn(Task::new_kernel(test_kernel_task, 0xbeef));
    m.spawn(Task::new_user("user_shell"));

    TASK_INITED.store(true, Ordering::SeqCst);
}

pub fn current<'a>() -> CurrentTask<'a> {
    CurrentTask::get()
}

pub fn handle_irq(vector: usize) {
    let curr = current();
    curr.clear_need_resched();
    crate::drivers::interrupt::handle_irq(vector);
    if curr.inner_exclusive_access().need_resched() {
        curr.yield_now();
    }
}

pub fn timer_tick_periodic() {
    TASK_MANAGER.lock().scheduler_timer_tick();
}

pub fn spawn_task(task: Arc<Task>) {
    TASK_MANAGER.lock().spawn(task);
}

pub fn run() -> ! {
    println!("Running tasks...");
    instructions::enable_irqs();
    current().yield_now(); // current task is idle at this time
    loop {
        current().yield_now();
        // instructions::wait_for_ints(); // the `HLT` instruction has poor performance
    }
}
