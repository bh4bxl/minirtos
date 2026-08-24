use crate::{arch, sched, task::TaskId};

use super::{CriticalSection, CriticalSectionLock, WaitQueue, critical_section};

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

pub(crate) struct Mutex {
    inner: CriticalSectionLock<MutexInner>,
}

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

    /// Blocking lock acquisition (thread context only).
    pub fn lock(&self) {
        loop {
            let acquired = critical_section(|cs| self.lock_cs(cs));
            if acquired {
                return;
            }
            arch::request_context_switch();
        }
    }

    /// Non-blocking lock acquisition.
    pub fn try_lock(&self) -> bool {
        critical_section(|cs| self.try_lock_cs(cs))
    }

    pub fn try_lock_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            if inner.locked {
                return false;
            }

            let sched = sched::scheduler();
            let id = sched.current_task_id(cs);

            inner.locked = true;
            inner.owner = Some(id);
            inner.handoff_to = None;

            sched.mutex_acquired(cs, id);

            true
        })
    }

    pub fn lock_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            let sched = sched::scheduler();
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
            if inner.owner == Some(id) && inner.handoff_to == Some(id) {
                inner.handoff_to = None;
                return true;
            }

            // The mutex is intentionally non-recursive.
            if inner.owner == Some(id) {
                panic!("recursive mutex lock by task {}", id.raw());
            }

            inner.waiters.block_current(cs);
            false
        })
    }

    /// Release the mutex.
    pub fn unlock(&self) {
        let need_reschedule = critical_section(|cs| self.unlock_cs(cs));

        if need_reschedule {
            arch::request_context_switch();
        }
    }

    pub fn unlock_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| {
            let sched = sched::scheduler();
            let owner = sched.current_task_id(cs);

            if inner.owner != Some(owner) {
                panic!("mutex unlock by non-owner: task={}", owner.raw());
            }

            sched.mutex_released(cs, owner);

            if let Some(next) = inner.waiters.pop_one() {
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
    }

    pub fn is_locked(&self) -> bool {
        critical_section(|cs| self.is_locked_cs(cs))
    }

    pub fn is_locked_cs(&self, cs: &CriticalSection) -> bool {
        self.inner.lock(cs, |inner| inner.locked)
    }

    pub fn owner(&self) -> Option<TaskId> {
        critical_section(|cs| self.owner_cs(cs))
    }

    pub fn owner_cs(&self, cs: &CriticalSection) -> Option<TaskId> {
        self.inner.lock(cs, |inner| inner.owner)
    }

    pub fn waiter_count(&self) -> usize {
        critical_section(|cs| self.waiter_count_cs(cs))
    }

    pub fn waiter_count_cs(&self, cs: &CriticalSection) -> usize {
        self.inner.lock(cs, |inner| inner.waiters.len())
    }
}
