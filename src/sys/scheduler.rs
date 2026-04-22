use crate::sys::synchronization::interface::Mutex;
use crate::sys::task::{Priority, TaskEntry};

use super::synchronization::IrqSafeNullLock;
use super::task::{TaskControlBlock, TaskId};

pub mod interface {
    use crate::sys::task::{Priority, TaskEntry, TaskId};

    pub trait Scheduler {
        fn add_task(
            &self,
            entry: TaskEntry,
            arg: *mut (),
            priority: Priority,
            name: &'static str,
        ) -> Result<TaskId, &'static str>;

        fn current_task_sp(&self) -> *mut u32;
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
    fn add_task(
        &self,
        entry: TaskEntry,
        arg: *mut (),
        priority: Priority,
        name: &'static str,
    ) -> Result<TaskId, &'static str> {
        self.inner.lock(|inner| {
            for slot in inner.tasks.iter_mut() {
                if slot.is_none() {
                    *slot = Some(TaskControlBlock::new(entry, arg, priority, name));

                    let task = slot.as_mut().unwrap();
                    task.sp = task.init_stack(task.entry, task.arg);

                    inner.task_count += 1;

                    return Ok(task.id);
                }
            }
            Err("Task table is full")
        })
    }

    fn current_task_sp(&self) -> *mut u32 {
        self.inner
            .lock(|inner| inner.tasks[inner.current].as_ref().unwrap().sp)
    }
}

static CURR_SCHEDULER: Scheduler = Scheduler::new();

pub fn scheduler() -> &'static dyn interface::Scheduler {
    &CURR_SCHEDULER
}
