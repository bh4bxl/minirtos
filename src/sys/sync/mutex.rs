use heapless::Vec;

use crate::sys::{
    arch::arm_cortex_m::trigger_pendsv,
    scheduler,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
    syscall,
    task::TaskId,
};

struct MutexInner {
    locked: bool,
    owner: Option<TaskId>,
    waiters: Vec<TaskId, 16>,
}

pub struct Mutex {
    inner: CriticalSectionLock<MutexInner>,
}

#[allow(dead_code)]
impl Mutex {
    pub const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(MutexInner {
                locked: false,
                owner: None,
                waiters: Vec::new(),
            }),
        }
    }

    /// Thread context only
    pub fn lock(&self) {
        loop {
            let acquired = critical_section(|cs| self.lock_cs(cs));
            if acquired {
                return;
            }

            syscall::yield_now();
        }
    }

    /// ISR-safe / non-blocking
    pub fn try_lock(&self) -> bool {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if !inner.locked {
                    inner.locked = true;
                    inner.owner = Some(scheduler::scheduler().current_task_id(cs));
                    true
                } else {
                    false
                }
            })
        })
    }

    fn lock_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            let sched = scheduler::scheduler();
            let id = sched.current_task_id(cs);

            if !inner.locked {
                inner.locked = true;
                inner.owner = Some(id);
                true
            } else {
                if !inner.waiters.iter().any(|&x| x == id) {
                    let _ = inner.waiters.push(id);
                }

                sched.block_current_task(cs);
                false
            }
        })
    }

    pub fn unlock(&self) {
        let should_reschedule = critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                let sched = scheduler::scheduler();
                let id = sched.current_task_id(cs);

                if inner.owner != Some(id) {
                    return false;
                }

                inner.locked = false;
                inner.owner = None;

                if let Some(next) = inner.waiters.pop() {
                    sched.wake_task(cs, next);
                    true
                } else {
                    inner.locked = false;
                    inner.owner = None;
                    false
                }
            })
        });

        if should_reschedule {
            trigger_pendsv();
        }
    }
}
