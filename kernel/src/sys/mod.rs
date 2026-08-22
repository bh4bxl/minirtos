use crate::SysError;

mod sync;
mod task;

pub use sync::event::Event;
pub use sync::message_queue::MessageQueue;
pub use sync::mutex::Mutex;
pub use sync::semaphore::Semaphore;
pub(crate) use sync::sync_dispatch;
pub(crate) use task::task_dispatch;
pub use task::{exit, get_tick, sleep_ms, yield_now};

#[repr(u8)]
#[derive(Clone, Copy)]
pub(crate) enum SyscallId {
    StartFirst = 0,
    Task = 1,
    Sync = 2,
}

impl TryFrom<u8> for SyscallId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::StartFirst),
            1 => Ok(Self::Task),
            2 => Ok(Self::Sync),
            _ => Err(()),
        }
    }
}

pub(crate) enum SyscallResult {
    None,
    U32(u32),
    U64(u64),
    Error(SysError),
}
