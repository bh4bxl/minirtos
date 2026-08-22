use super::{CriticalSection, CriticalSectionLock, WaitQueue, critical_section};

struct EventInner {
    signaled: bool,
    waiters: WaitQueue,
}

pub(crate) struct Event {
    inner: CriticalSectionLock<EventInner>,
}

impl Event {
    pub const fn new(initially_signaled: bool) -> Self {
        Self {
            inner: CriticalSectionLock::new(EventInner {
                signaled: initially_signaled,
                waiters: WaitQueue::new(),
            }),
        }
    }

    pub fn wait(&self) {
        loop {
            let signaled = critical_section(|cs| self.wait_cs(cs));
            if signaled {
                return;
            }
        }
    }

    pub fn signal(&self) {
        critical_section(|cs| self.signal_cs(cs));
    }

    pub fn is_signaled(&self) -> bool {
        critical_section(|cs| self.is_signaled_cs(cs))
    }

    pub fn wait_cs(&self, cs: &CriticalSection) -> bool {
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

    pub fn signal_cs(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            if !inner.waiters.wake_one(cs) {
                inner.signaled = true;
            }
        });
    }

    pub fn is_signaled_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| inner.signaled)
    }
}
