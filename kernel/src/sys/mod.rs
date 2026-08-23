use crate::SysError;

mod ipc;
mod sync;
mod task;

pub use sync::event::Event;
pub use sync::mutex::Mutex;
pub use sync::semaphore::Semaphore;
pub(crate) use sync::sync_dispatch;
pub(crate) use task::task_dispatch;
pub use task::{exit, get_tick, sleep_ms, yield_now};

pub(crate) use minirtos_abi::SyscallId;

pub(crate) enum SyscallResult {
    None,
    U32(u32),
    U64(u64),
    Error(SysError),
}
