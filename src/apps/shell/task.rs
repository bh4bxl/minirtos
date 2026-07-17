use crate::apps::shell::ShellApp;
use crate::println;
use crate::sys::scheduler;
use crate::sys::task::Priority;

const TASKS_PRIO: u8 = 100;
const TASKS_STACK_SIZE: usize = 256;

extern "C" fn tasks_task(_arg: *mut ()) {
    let tasks = scheduler::scheduler().tasks();

    println!("ID   Name       State        Prio   Stack");

    for task in tasks {
        println!(
            "{:<4} {:<10} {:<12} {:<6} {}/{}",
            task.id.0,
            task.name,
            task.state.as_str(),
            task.priority.0,
            task.stack_used,
            task.stack_total,
        );
    }
}

pub(super) static TASKS_APP: ShellApp = ShellApp::new(
    "tasks",
    "Show task list",
    tasks_task,
    TASKS_STACK_SIZE,
    Priority(TASKS_PRIO),
);
