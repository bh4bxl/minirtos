use super::super::{
    arch::arm_cortex_m,
    scheduler,
    sync::wait_queue::WaitQueue,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
    task::TaskId,
};

struct MutexInner {
    locked: bool,
    owner: Option<TaskId>,

    /// Set during direct handoff.
    ///
    /// This distinguishes a task receiving ownership from the same task
    /// recursively trying to lock a non-recursive mutex.
    handoff_to: Option<TaskId>,
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
                handoff_to: None,
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

            /*
             * block_current() has marked this task Blocked and pended PendSV.
             * Ensure the processor observes the pending exception immediately
             * after leaving the critical section.
             */
            cortex_m::asm::isb();
        }
    }

    /// Task context, non-blocking.
    pub fn try_lock(&self) -> bool {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if inner.locked {
                    return false;
                }

                let sched = scheduler::scheduler();
                let id = sched.current_task_id(cs);

                inner.locked = true;
                inner.owner = Some(id);
                inner.handoff_to = None;

                sched.mutex_acquired(cs, id);

                true
            })
        })
    }

    fn lock_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            let sched = scheduler::scheduler();
            let id = sched.current_task_id(cs);

            if !inner.locked {
                debug_assert!(inner.owner.is_none());
                debug_assert!(inner.handoff_to.is_none());

                inner.locked = true;
                inner.owner = Some(id);

                sched.mutex_acquired(cs, id);

                return true;
            }

            // Complete a direct handoff.
            //
            // unlock() has already:
            //   - transferred owner to this task
            //   - increased this task's owned_mutex_count
            //   - woken this task
            if inner.owner == Some(id) && inner.handoff_to == Some(id) {
                inner.handoff_to = None;
                return true;
            }

            // The mutex is intentionally non-recursive.
            if inner.owner == Some(id) {
                panic!("recursive mutex lock by task {}", id.0);
            }

            inner.waiters.block_current(cs);
            false
        })
    }

    pub fn unlock(&self) {
        let need_reschedule = critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                let sched = scheduler::scheduler();
                let owner = sched.current_task_id(cs);

                if inner.owner != Some(owner) {
                    panic!("mutex unlock by non-owner: task={}", owner.0);
                }

                // Remove ownership from the current owner first.
                sched.mutex_released(cs, owner);

                if let Some(next) = inner.waiters.pop_one() {
                    // Direct handoff:
                    //
                    // The mutex never becomes unlocked between the two tasks.
                    // This prevents another task from stealing it before the
                    // selected waiter resumes.
                    inner.locked = true;
                    inner.owner = Some(next);
                    inner.handoff_to = Some(next);

                    sched.mutex_acquired(cs, next);
                    sched.wake_task(cs, next);

                    true
                } else {
                    inner.locked = false;
                    inner.owner = None;
                    inner.handoff_to = None;

                    false
                }
            })
        });

        if need_reschedule {
            arm_cortex_m::trigger_pendsv();
            cortex_m::asm::isb();
        }
    }

    pub fn is_locked(&self) -> bool {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.locked))
    }

    pub fn owner(&self) -> Option<TaskId> {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.owner))
    }

    pub fn waiter_count(&self) -> usize {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.waiters.len()))
    }
}
