use console::queue_console::{QueueConsole, queue_console_task};

pub mod arch;
pub mod board;
pub mod console;
pub mod debug_info;
pub mod device_driver;
pub mod input;
pub mod interrupt;
pub mod memory;
pub mod print;
pub mod scheduler;
pub mod sync;
pub mod synchronization;
pub mod syscall;
pub mod task;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SysError {
    InvalidArgument = -1,
    NoMemory = -2,
    NoResource = -3,
    Busy = -4,
    Timeout = -5,
    WouldBlock = -6,
    NotFound = -7,
    AlreadyExists = -8,
    NotSupported = -9,

    InvalidState = -20,

    Io = -40,
    DeviceError = -41,
    ProtocolError = -42,
}

static QUEUE_CONSOLE: QueueConsole = QueueConsole::new();

const QUEUE_CONSOLE_STACK_SIZE: usize = 128;
// static QUEUE_CONSOLE_STACK: task::TaskStack<QUEUE_CONSOLE_STACK_SIZE> = task::TaskStack::new();

pub fn kernel_init() -> Result<(), SysError> {
    scheduler::init();

    // Register QueueConsole
    defmt::info!("Registering console");

    let mut qcon = task::Task::<QUEUE_CONSOLE_STACK_SIZE>::new(queue_console_task)
        .priority(task::Priority(200))
        .name("console");

    qcon.run()?;

    console::register_console(&QUEUE_CONSOLE);

    Ok(())
}
