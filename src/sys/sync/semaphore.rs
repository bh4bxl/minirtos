use crate::sys::{
    sync::wait_queue::WaitQueue,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
};

struct SemaphoreInner {
    count: isize,
    waiters: WaitQueue,
}

pub struct Semaphore {
    inner: CriticalSectionLock<SemaphoreInner>,
}

#[allow(dead_code)]
impl Semaphore {
    pub const fn new(initial: isize) -> Self {
        Self {
            inner: CriticalSectionLock::new(SemaphoreInner {
                count: initial,
                waiters: WaitQueue::new(),
            }),
        }
    }

    /// Blocking wait (thread context only)
    pub fn wait(&self) {
        loop {
            let acquired = critical_section(|cs| self.wait_cs(cs));
            if acquired {
                return;
            }
        }
    }

    /// Non-blocking try_wait (ISR-safe)
    pub fn try_wait(&self) -> bool {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if inner.count > 0 {
                    inner.count -= 1;
                    true
                } else {
                    false
                }
            })
        })
    }

    fn wait_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            if inner.count > 0 {
                inner.count -= 1;
                true
            } else {
                inner.waiters.block_current(cs);
                false
            }
        })
    }

    /// Signal / post
    pub fn signal(&self) {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                inner.count += 1;
                inner.waiters.wake_one(cs);
            });
        });
    }

    pub fn available(&self) -> isize {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.count))
    }
}
