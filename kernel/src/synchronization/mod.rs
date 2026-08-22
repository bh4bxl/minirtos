use crate::arch;

mod critical_section;
mod event;
pub(crate) mod interface;
mod lock;
mod message_queue;
mod mutex;
mod semaphore;
mod wait_queue;

pub(crate) use event::Event;
pub(crate) use lock::{CriticalSectionLock, InitStateLock, IrqLock, NullLock};
pub(crate) use message_queue::{MessageQueue, SendResult};
pub(crate) use mutex::Mutex;
pub(crate) use semaphore::Semaphore;
use wait_queue::WaitQueue;

struct IrqGuard {
    state: arch::IrqState,
}

impl IrqGuard {
    #[inline]
    fn new() -> Self {
        Self {
            state: arch::disable_interrupts(),
        }
    }
}

impl Drop for IrqGuard {
    #[inline]
    fn drop(&mut self) {
        arch::restore_interrupts(self.state);
    }
}

pub(crate) struct CriticalSection(());

pub(crate) fn critical_section<R>(f: impl FnOnce(&CriticalSection) -> R) -> R {
    let _guard = IrqGuard::new();

    let cs = CriticalSection(());
    f(&cs)
}
