use heapless::Vec;

use crate::sys::{
    arch::arm_cortex_m::trigger_pendsv,
    scheduler,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
    syscall,
    task::TaskId,
};

const MAX_WAITERS: usize = 16;

struct SemaphoreInner {
    count: i32,
    waiters: Vec<TaskId, MAX_WAITERS>,
}

pub struct Semaphore {
    inner: CriticalSectionLock<SemaphoreInner>,
}

#[allow(dead_code)]
impl Semaphore {
    pub const fn new(initial: i32) -> Self {
        Self {
            inner: CriticalSectionLock::new(SemaphoreInner {
                count: initial,
                waiters: Vec::new(),
            }),
        }
    }

    /// Thread context only.
    pub fn wait(&self) {
        loop {
            let acquired = critical_section(|cs| self.wait_cs(cs));
            if acquired {
                return;
            }

            syscall::yield_now();
        }
    }

    /// ISR-safe / non-blocking.
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
                let task_id = scheduler::scheduler().current_task_id(cs);

                // Avoid duplicate enqueue if wait() loops after being woken incorrectly.
                if !inner.waiters.iter().any(|&id| id == task_id) {
                    let _ = inner.waiters.push(task_id);
                }

                scheduler::scheduler().block_current_task(cs);
                false
            }
        })
    }

    /// Signal / post
    pub fn signal(&self) {
        let should_reschedule = critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                inner.count += 1;
                if let Some(task_id) = inner.waiters.pop() {
                    scheduler::scheduler().wake_task(cs, task_id);
                    true
                } else {
                    false
                }
            })
        });

        if should_reschedule {
            trigger_pendsv();
        }
    }
}
