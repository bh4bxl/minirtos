use crate::sys::arch::arm_cortex_m::trigger_pendsv;
use crate::sys::synchronization::{CriticalSection, CriticalSectionLock};
use crate::sys::task::{Priority, TaskEntry, TaskState};

use super::task::{TaskControlBlock, TaskId};

//#[allow(dead_code)]
pub mod interface {
    use crate::sys::{
        synchronization::CriticalSection,
        task::{Priority, TaskEntry, TaskId},
    };

    pub trait Scheduler {
        fn add_task(
            &self,
            cs: &CriticalSection,
            entry: TaskEntry,
            arg: *mut (),
            priority: Priority,
            name: &'static str,
        ) -> Result<TaskId, &'static str>;

        fn current_task_sp(&self, cs: &CriticalSection) -> *mut u32;

        fn current_task_sleep(&self, cs: &CriticalSection, ms: u32);

        fn update_tick(&self, cs: &CriticalSection);

        fn start(&self, cs: &CriticalSection);

        fn get_tick(&self, cs: &CriticalSection) -> u64;

        // No CriticalSection
        unsafe fn switch(&self, old_sp: *mut u32) -> *mut u32;
    }
}

pub const MAX_TASKS: usize = 16;

struct SchedulerInner {
    pub tasks: [Option<TaskControlBlock>; MAX_TASKS],
    pub current: usize,
    pub task_count: usize,
    tick_count: u64,
    started: bool,
}

impl SchedulerInner {
    const fn new() -> Self {
        Self {
            tasks: [const { None }; MAX_TASKS],
            current: 0,
            task_count: 0,
            tick_count: 0,
            started: false,
        }
    }

    fn next_task(&self) -> usize {
        // Find highest priority among Ready tasks.
        let best_prio = self
            .tasks
            .iter()
            .flatten()
            .filter(|t| t.state == TaskState::Ready)
            .map(|t| t.priority)
            .min()
            .unwrap_or(Priority(255));

        // Pick the one *after* current (round-robin)
        let start = (self.current + 1) % MAX_TASKS;
        for i in 0..MAX_TASKS {
            let idx = (start + i) % MAX_TASKS;
            if let Some(ref t) = self.tasks[idx] {
                if t.state == TaskState::Ready && t.priority == best_prio {
                    return idx;
                }
            }
        }

        0
    }
}

struct Scheduler {
    inner: CriticalSectionLock<SchedulerInner>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(SchedulerInner::new()),
        }
    }
}

impl interface::Scheduler for Scheduler {
    fn add_task(
        &self,
        cs: &CriticalSection,
        entry: TaskEntry,
        arg: *mut (),
        priority: Priority,
        name: &'static str,
    ) -> Result<TaskId, &'static str> {
        self.inner.lock(cs, |inner| {
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

    fn current_task_sp(&self, cs: &CriticalSection) -> *mut u32 {
        self.inner
            .lock(cs, |inner| inner.tasks[inner.current].as_ref().unwrap().sp)
    }

    fn current_task_sleep(&self, cs: &CriticalSection, ms: u32) {
        self.inner.lock(cs, |inner| {
            let wake = inner.tick_count + ms as u64;
            if let Some(task) = &mut inner.tasks[inner.current] {
                task.state = TaskState::Sleeping;
                task.wake_tick = wake;
            }
        });
    }

    fn update_tick(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            inner.tick_count += 1;

            // Wake any sleeping tasks whose deadline has passed.
            for task in inner.tasks.iter_mut().flatten() {
                if task.state == TaskState::Sleeping && task.wake_tick <= inner.tick_count {
                    task.state = TaskState::Ready;
                }
            }

            // Protect for not ready
            if inner.task_count == 0 || !inner.started {
                return;
            }

            // Rrigger PendSV
            if inner.tick_count % 10 == 0 {
                trigger_pendsv();
            }
        })
    }

    fn start(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            inner.started = true;

            if inner.task_count > 0 {
                inner.current = 0;
                if let Some(task) = &mut inner.tasks[0] {
                    task.state = TaskState::Running;
                }
            }
        })
    }

    fn get_tick(&self, cs: &CriticalSection) -> u64 {
        self.inner.lock(cs, |inner| inner.tick_count)
    }

    unsafe fn switch(&self, old_sp: *mut u32) -> *mut u32 {
        unsafe {
            self.inner.lock_unchecked(|inner| {
                // Save SP of the running task.
                if let Some(ref mut task) = inner.tasks[inner.current] {
                    task.sp = old_sp;
                    if task.state == TaskState::Running {
                        task.state = TaskState::Ready;
                    }
                }

                // Pick next runnable task
                inner.current = inner.next_task();

                if let Some(ref mut task) = inner.tasks[inner.current] {
                    task.state = TaskState::Running;
                    task.sp
                } else {
                    // Swtich to idle task
                    inner.current = 0;
                    inner.tasks[inner.current].as_mut().unwrap().state = TaskState::Running;
                    inner.tasks[inner.current].as_ref().unwrap().sp
                }
            })
        }
    }
}

static CURR_SCHEDULER: Scheduler = Scheduler::new();

pub fn scheduler() -> &'static dyn interface::Scheduler {
    &CURR_SCHEDULER
}

/// Called from PendSV handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_switch(old_sp: *mut u32) -> *mut u32 {
    unsafe { scheduler().switch(old_sp) }
}
