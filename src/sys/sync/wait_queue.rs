use heapless::Vec;

use super::super::{arch::arm_cortex_m, scheduler, synchronization::CriticalSection, task::TaskId};

const MAX_WAITERS: usize = 8;

pub struct WaitQueue {
    waiters: Vec<TaskId, MAX_WAITERS>,
}

#[allow(dead_code)]
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
        let sched = scheduler::scheduler();
        let tid = sched.current_task_id(cs);

        // A task must appear at most once in a wait queue.
        if !self.waiters.contains(&tid) {
            if self.waiters.push(tid).is_err() {
                panic!("wait queue full");
            }
        }

        sched.block_current_task(cs);
        arm_cortex_m::trigger_pendsv();
    }

    /// Remove and wake the oldest waiting task.
    pub fn wake_one(&mut self, cs: &CriticalSection) -> bool {
        let Some(tid) = self.pop_one() else {
            return false;
        };

        scheduler::scheduler().wake_task(cs, tid);
        arm_cortex_m::trigger_pendsv();

        true
    }

    /// Wake every task currently waiting.
    ///
    /// Returns the number of tasks woken.
    pub fn wake_all(&mut self, cs: &CriticalSection) -> usize {
        let sched = scheduler::scheduler();
        let mut count = 0;

        while let Some(tid) = self.pop_one() {
            sched.wake_task(cs, tid);
            count += 1;
        }

        if count != 0 {
            arm_cortex_m::trigger_pendsv();
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
