use crate::sys::synchronization::{CriticalSection, CriticalSectionLock, critical_section};

use super::wait_queue::WaitQueue;

pub struct EventInner {
    signaled: bool,
    waiters: WaitQueue,
}

pub struct Event {
    inner: CriticalSectionLock<EventInner>,
}

#[allow(dead_code)]
impl Event {
    pub const fn new(signaled: bool) -> Self {
        Self {
            inner: CriticalSectionLock::new(EventInner {
                signaled,
                waiters: WaitQueue::new(),
            }),
        }
    }

    pub fn wait(&self) {
        loop {
            let acquired = critical_section(|cs| self.wait_cs(cs));
            if acquired {
                return;
            }
        }
    }

    pub fn signal(&self) {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                inner.signaled = true;
                inner.waiters.wake_one(cs);
            });
        });
    }

    pub fn is_signaled(&self) -> bool {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.signaled))
    }

    fn wait_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            if inner.signaled {
                inner.signaled = false;
                true
            } else {
                inner.waiters.block_current(cs);
                false
            }
        })
    }

    fn signal_cs(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            if inner.waiters.wake_one(cs) {
                return;
            }

            inner.signaled = true;
        });
    }
}
