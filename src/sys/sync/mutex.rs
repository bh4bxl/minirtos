use crate::sys::{
    arch::arm_cortex_m,
    scheduler,
    sync::wait_queue::WaitQueue,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
    task::TaskId,
};

struct MutexInner {
    locked: bool,
    owner: Option<TaskId>,
    waiters: WaitQueue,
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
                waiters: WaitQueue::new(),
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
        }
    }

    /// ISR-safe / non-blocking
    pub fn try_lock(&self) -> bool {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if inner.locked {
                    return false;
                }

                inner.locked = true;
                inner.owner = Some(scheduler::scheduler().current_task_id(cs));
                true
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
                return true;
            }

            if inner.owner == Some(id) {
                return true;
            }

            inner.waiters.block_current(cs);
            false
        })
    }

    pub fn unlock(&self) {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                let sched = scheduler::scheduler();
                let id = sched.current_task_id(cs);

                if inner.owner != Some(id) {
                    return;
                }

                if let Some(next) = inner.waiters.pop_one() {
                    // Direct handoff: mutex stays locked, owner becomes next task.
                    inner.owner = Some(next);
                    sched.wake_task(cs, next);
                    arm_cortex_m::trigger_pendsv();
                } else {
                    inner.locked = false;
                    inner.owner = None;
                }
            });
        });
    }

    pub fn is_locked(&self) -> bool {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.locked))
    }
}
