mod event;
mod interface;
mod lock;
mod mutex;
mod semaphore;
mod wait_queue;

pub(crate) use event::Event;
pub(crate) use interface::ReadWriteEx;
pub(crate) use lock::{
    CriticalSection, CriticalSectionLock, InitStateLock, NullLock, critical_section,
};
pub(crate) use mutex::Mutex;
pub(crate) use semaphore::Semaphore;
pub(crate) use wait_queue::WaitQueue;

pub use interface::Lock;
pub use lock::IrqLock;
