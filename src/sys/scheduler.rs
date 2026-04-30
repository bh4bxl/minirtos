use core::sync::atomic::{AtomicBool, Ordering};

use crate::sys::arch::arm_cortex_m::trigger_pendsv;
use crate::sys::synchronization::{CriticalSection, CriticalSectionLock, critical_section};
use crate::sys::task::{Priority, TaskEntry, TaskStack, TaskState};

use super::task::{TaskControlBlock, TaskId};

#[allow(dead_code)]
pub mod interface {
    use crate::sys::{
        synchronization::CriticalSection,
        task::{Priority, TaskEntry, TaskId, TaskState},
    };

    pub trait Scheduler {
        fn init(&self, cs: &CriticalSection);

        fn add_task(
            &self,
            cs: &CriticalSection,
            entry: TaskEntry,
            arg: *mut (),
            stack: &'static mut [u32],
            priority: Priority,
            name: &'static str,
        ) -> Result<TaskId, &'static str>;

        fn current_task_sp(&self, cs: &CriticalSection) -> *mut u32;

        fn current_task_id(&self, cs: &CriticalSection) -> TaskId;

        fn set_current_task_status(&self, cs: &CriticalSection, state: TaskState);

        fn current_task_sleep(&self, cs: &CriticalSection, ms: u32);

        fn block_current_task(&self, cs: &CriticalSection);

        fn wake_task(&self, cs: &CriticalSection, id: TaskId);

        fn update_tick(&self, cs: &CriticalSection);

        fn start(&self, cs: &CriticalSection);

        fn get_tick(&self, cs: &CriticalSection) -> u64;

        fn dump_tasks(&self) {}

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

const IDLE_TASK_ID: usize = 0;
const IDLE_STACK_SIZE: usize = 64;
static IDLE_STACK: TaskStack<IDLE_STACK_SIZE> = TaskStack::new();

extern "C" fn idle_task_entry(_arg: *mut ()) -> ! {
    loop {
        cortex_m::asm::wfi();
    }
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

        IDLE_TASK_ID
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
    fn init(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            if inner.tasks[IDLE_TASK_ID].is_none() {
                inner.tasks[IDLE_TASK_ID] = Some(
                    TaskControlBlock::new(
                        idle_task_entry,
                        core::ptr::null_mut(),
                        IDLE_STACK.get(),
                        Priority(255),
                        "idle",
                    )
                    .with_time_slice(1),
                );

                let idle = inner.tasks[IDLE_TASK_ID].as_mut().unwrap();

                idle.sp = idle.init_stack(idle.entry, idle.arg);
                idle.state = TaskState::Ready;

                inner.task_count += 1;
            }
        })
    }

    fn add_task(
        &self,
        cs: &CriticalSection,
        entry: TaskEntry,
        arg: *mut (),
        stack: &'static mut [u32],
        priority: Priority,
        name: &'static str,
    ) -> Result<TaskId, &'static str> {
        self.inner.lock(cs, |inner| {
            for slot in inner.tasks.iter_mut() {
                if slot.is_none() {
                    *slot = Some(TaskControlBlock::new(entry, arg, stack, priority, name));

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

    fn current_task_id(&self, cs: &CriticalSection) -> TaskId {
        self.inner
            .lock(cs, |inner| inner.tasks[inner.current].as_ref().unwrap().id)
    }

    fn set_current_task_status(&self, cs: &CriticalSection, state: TaskState) {
        self.inner.lock(cs, |inner| {
            if let Some(task) = &mut inner.tasks[inner.current] {
                task.state = state;
            }
        })
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

    fn block_current_task(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            if let Some(task) = &mut inner.tasks[inner.current] {
                task.state = TaskState::Blocked;
            }
        });
    }

    fn wake_task(&self, cs: &CriticalSection, id: TaskId) {
        self.inner.lock(cs, |inner| {
            for task in inner.tasks.iter_mut().flatten() {
                if task.id == id {
                    if task.state == TaskState::Blocked {
                        task.state = TaskState::Ready;
                    }
                    break;
                }
            }
        });
    }

    fn update_tick(&self, cs: &CriticalSection) {
        let need_switch = self.inner.lock(cs, |inner| {
            inner.tick_count += 1;
            let now = inner.tick_count;

            // Wake sleeping tasks whose deadline has passed
            for task in inner.tasks.iter_mut().flatten() {
                if task.state == TaskState::Sleeping && task.wake_tick <= now {
                    task.state = TaskState::Ready;
                }
            }

            // No scheduling before system start
            if inner.task_count == 0 || !inner.started {
                return false;
            }

            let current = inner.current;
            let Some(task) = inner.tasks[current].as_mut() else {
                return false;
            };

            // Only apply time slicing to running task
            if task.state != TaskState::Running {
                return false;
            }

            // Decrement remaining time slice
            task.remaining_slice = task.remaining_slice.saturating_sub(1);

            // Time slice expired → request context switch
            if task.remaining_slice == 0 {
                task.remaining_slice = task.time_slice;
                true
            } else {
                false
            }
        });

        if need_switch {
            trigger_pendsv();
        }
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

    fn dump_tasks(&self) {
        use crate::print;
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                print!("ID   Name       State     Prio   Stack\r\n");

                for (i, task_opt) in inner.tasks.iter().enumerate() {
                    if let Some(task) = task_opt {
                        let used = task.stack_used_bytes();
                        let total = task.stack_total_bytes();
                        let state_str = match task.state {
                            TaskState::Ready => "Ready",
                            TaskState::Running => "Running",
                            TaskState::Blocked => "Blocked",
                            TaskState::Sleeping => "Sleep",
                            TaskState::Suspended => "Suspended",
                            TaskState::Terminated => "Terminated",
                        };

                        print!(
                            "{:<4} {:<10} {:<9} {:<6} {}/{}\r\n",
                            i, task.name, state_str, task.priority.0, used, total
                        );
                    }
                }
            });
        });
    }

    unsafe fn switch(&self, old_sp: *mut u32) -> *mut u32 {
        unsafe {
            self.inner.lock_unchecked(|inner| {
                // Save SP of the running task.
                if let Some(ref mut task) = inner.tasks[inner.current] {
                    task.sp = old_sp;

                    // Check stack after saving the latest SP.
                    task.check_stack_guard();

                    if task.state == TaskState::Running {
                        task.state = TaskState::Ready;
                    }
                }

                // Pick next runnable task
                inner.current = inner.next_task();

                if let Some(ref mut task) = inner.tasks[inner.current] {
                    task.state = TaskState::Running;

                    // Reset the time slice if it was exhausted.
                    if task.remaining_slice == 0 {
                        task.remaining_slice = task.time_slice;
                    }

                    task.sp
                } else {
                    // Switch to idle task
                    inner.current = 0;
                    let task = inner.tasks[inner.current].as_mut().unwrap();
                    task.state = TaskState::Running;

                    if task.remaining_slice == 0 {
                        task.remaining_slice = task.time_slice;
                    }

                    task.sp
                }
            })
        }
    }
}

static CURR_SCHEDULER: Scheduler = Scheduler::new();

pub fn scheduler() -> &'static dyn interface::Scheduler {
    &CURR_SCHEDULER
}

static SCHEDULER_INITIALIZED: AtomicBool = AtomicBool::new(false);
pub fn init() {
    if SCHEDULER_INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }

    critical_section(|cs| {
        scheduler().init(cs);
    });
}

/// Called from PendSV handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_switch(old_sp: *mut u32) -> *mut u32 {
    unsafe { scheduler().switch(old_sp) }
}
