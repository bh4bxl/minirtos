use crate::sys::console::queue_console::{QueueConsole, queue_console_task};

pub mod arch;
pub mod board;
pub mod console;
pub mod debug_info;
pub mod device_driver;
pub mod input;
pub mod interrupt;
pub mod print;
pub mod scheduler;
pub mod sync;
pub mod synchronization;
pub mod syscall;
pub mod task;

static QUEUE_CONSOLE: QueueConsole = QueueConsole::new();

pub fn kernel_init() -> Result<(), &'static str> {
    scheduler::init();

    // Register QueueConsole
    defmt::info!("Registering console");
    if let Err(x) = syscall::thread_create(
        queue_console_task,
        core::ptr::null_mut(),
        task::Priority(200),
        "console",
    ) {
        return Err(x);
    }

    console::register_console(&QUEUE_CONSOLE);

    Ok(())
}
