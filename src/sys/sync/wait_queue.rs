use heapless::Vec;

use crate::sys::{arch::arm_cortex_m, scheduler, synchronization::CriticalSection, task::TaskId};

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

    pub fn block_current(&mut self, cs: &CriticalSection) {
        let sched = scheduler::scheduler();
        let tid = sched.current_task_id(cs);

        if !self.waiters.iter().any(|&id| id == tid) {
            let _ = self.waiters.push(tid);
        }

        sched.block_current_task(cs);
        arm_cortex_m::trigger_pendsv();
    }

    pub fn wake_one(&mut self, cs: &CriticalSection) -> bool {
        if let Some(tid) = self.waiters.pop() {
            scheduler::scheduler().wake_task(cs, tid);
            arm_cortex_m::trigger_pendsv();
            true
        } else {
            false
        }
    }

    pub fn wake_all(&mut self, cs: &CriticalSection) {
        while let Some(tid) = self.waiters.pop() {
            scheduler::scheduler().wake_task(cs, tid);
        }

        arm_cortex_m::trigger_pendsv();
    }

    pub fn pop_one(&mut self) -> Option<TaskId> {
        self.waiters.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.waiters.is_empty()
    }
}
