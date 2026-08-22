use heapless::Vec;

use crate::{arch, sched, task::TaskId};

use super::CriticalSection;

const MAX_WAITERS: usize = 8;

pub(crate) struct WaitQueue {
    waiters: Vec<TaskId, MAX_WAITERS>,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            waiters: Vec::new(),
        }
    }

    /// Add the current task to the wait queue and mark it blocked.
    ///
    /// The pending PendSV will run after the caller leaves its critical section.
    pub fn block_current(&mut self, cs: &CriticalSection) {
        let sched = sched::scheduler();
        let tid = sched.current_task_id(cs);

        // A task must appear at most once in a wait queue.
        if !self.waiters.contains(&tid) {
            if self.waiters.push(tid).is_err() {
                panic!("wait queue full");
            }
        }

        sched.block_current_task(cs);
        arch::request_context_switch();
    }

    /// Remove and wake the oldest waiting task.
    pub fn wake_one(&mut self, cs: &CriticalSection) -> bool {
        let Some(tid) = self.pop_one() else {
            return false;
        };

        sched::scheduler().wake_task(cs, tid);
        arch::request_context_switch();

        true
    }

    /// Wake every task currently waiting.
    ///
    /// Returns the number of tasks woken.
    pub fn wake_all(&mut self, cs: &CriticalSection) -> usize {
        let sched = sched::scheduler();
        let mut count = 0;

        while let Some(tid) = self.pop_one() {
            sched.wake_task(cs, tid);
            count += 1;
        }

        if count != 0 {
            arch::request_context_switch();
        }

        count
    }

    /// Remove and return the oldest waiting task without waking it.
    pub fn pop_one(&mut self) -> Option<TaskId> {
        if self.waiters.is_empty() {
            None
        } else {
            // FIFO. MAX_WAITERS is small, so shifting is acceptable.
            Some(self.waiters.remove(0))
        }
    }

    /// Remove a specific task from this queue.
    ///
    /// Useful later for task cancellation and forced termination.
    pub fn remove(&mut self, tid: TaskId) -> bool {
        let Some(index) = self.waiters.iter().position(|&id| id == tid) else {
            return false;
        };

        self.waiters.remove(index);
        true
    }

    pub fn contains(&self, tid: TaskId) -> bool {
        self.waiters.contains(&tid)
    }

    pub fn len(&self) -> usize {
        self.waiters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.waiters.is_empty()
    }
}
