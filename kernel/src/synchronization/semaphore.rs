use super::{CriticalSection, CriticalSectionLock, WaitQueue, critical_section};

struct SemaphoreInner {
    count: isize,
    waiters: WaitQueue,
}

pub(crate) struct Semaphore {
    inner: CriticalSectionLock<SemaphoreInner>,
}

impl Semaphore {
    pub const fn new(initial: isize) -> Self {
        Self {
            inner: CriticalSectionLock::new(SemaphoreInner {
                count: initial,
                waiters: WaitQueue::new(),
            }),
        }
    }

    /// Blocking acquire (thread context only)
    pub fn acquire(&self) {
        loop {
            let acquired = critical_section(|cs| self.acquire_cs(cs));
            if acquired {
                return;
            }
        }
    }

    /// Non-blocking acquire (ISR-safe)
    pub fn try_acquire(&self) -> bool {
        critical_section(|cs| self.try_acquire_cs(cs))
    }

    pub(crate) fn acquire_cs(&self, cs: &CriticalSection) -> bool {
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

    pub(crate) fn try_acquire_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            if inner.count > 0 {
                inner.count -= 1;
                true
            } else {
                false
            }
        })
    }

    /// Release one permit.
    pub fn release(&self) {
        critical_section(|cs| self.release_cs(cs));
    }

    pub(crate) fn release_cs(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            inner.count += 1;
            inner.waiters.wake_one(cs);
        })
    }

    pub fn available_permits(&self) -> isize {
        critical_section(|cs| self.available_permits_cs(cs))
    }

    pub(crate) fn available_permits_cs(&self, cs: &CriticalSection) -> isize {
        self.inner.lock(cs, |inner| inner.count)
    }
}
