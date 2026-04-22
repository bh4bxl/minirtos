use crate::sys::synchronization::interface::Mutex;

use super::synchronization::IrqSafeNullLock;
use super::task::{TaskControlBlock, TaskId};

pub mod interface {
    use crate::sys::task::{TaskControlBlock, TaskId};

    pub trait Scheduler {
        fn add_task(&self, tcb: TaskControlBlock) -> Result<TaskId, &'static str>;

        fn current_task(&self) -> &mut TaskControlBlock;
    }
}

#[allow(dead_code)]
pub const MAX_TASKS: usize = 16;

struct SchedulerInner {
    tasks: [Option<TaskControlBlock>; MAX_TASKS],
    current: usize,
    task_count: u32,
}

impl SchedulerInner {
    const fn new() -> Self {
        Self {
            tasks: [const { None }; MAX_TASKS],
            current: 0,
            task_count: 0,
        }
    }
}

struct Scheduler {
    inner: IrqSafeNullLock<SchedulerInner>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            inner: IrqSafeNullLock::new(SchedulerInner::new()),
        }
    }
}

impl interface::Scheduler for Scheduler {
    fn add_task(&self, tcb: TaskControlBlock) -> Result<TaskId, &'static str> {
        self.inner.lock(|inner| {
            for solt in inner.tasks.iter_mut() {
                if solt.is_none() {
                    let id = tcb.id;
                    *solt = Some(tcb);
                    inner.task_count += 1;
                    return Ok(id);
                }
            }
            Err("Task table is full")
        })
    }

    fn current_task(&self) -> &mut TaskControlBlock {
        self.inner
            .lock(|inner| inner.tasks[inner.current].as_mut().unwrap())
    }
}

static CURR_SCHEDULER: Scheduler = Scheduler::new();

pub fn scheduler() -> &'static dyn interface::Scheduler {
    &CURR_SCHEDULER
}
