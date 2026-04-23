use crate::sys::arch::arm_cortex_m::trigger_pendsv;
use crate::sys::device_driver::{self, DeviceType};
use crate::sys::synchronization::NullLock;
use crate::sys::synchronization::interface::Mutex;
use crate::sys::task::{Priority, TaskEntry, TaskState};

use super::task::{TaskControlBlock, TaskId};

#[allow(dead_code)]
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

        fn update_tick(&self);

        fn start(&self);

        fn get_tick(&self) -> u64;

        fn switch(&self, old_sp: *mut u32) -> *mut u32;
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
        if self.current + 1 >= self.task_count {
            0
        } else {
            self.current + 1
        }
    }
}

struct Scheduler {
    inner: NullLock<SchedulerInner>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            inner: NullLock::new(SchedulerInner::new()),
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

    fn update_tick(&self) {
        self.inner.lock(|inner| {
            inner.tick_count += 1;

            // Wake any sleeping tasks whose deadline has passed.
            for task in inner.tasks.iter_mut().flatten() {
                if task.state == TaskState::Sleeping && task.wake_tick <= inner.tick_count {
                    task.state = TaskState::Ready;
                }
            }

            if inner.task_count == 0 || !inner.started {
                return;
            }

            // Rrigger PendSV
            if inner.tick_count % 10 == 0 {
                if let Some(gpio) = device_driver::driver_manager().open_device(DeviceType::Gpio, 0)
                {
                    let mut data = [19u8, 0];
                    if let Err(_x) = gpio.read(&mut data) {
                        defmt::error!("GPIO read failed: {}", data[0]);
                        return;
                    }

                    data[1] = if data[1] == 0 { 1 } else { 0 };

                    if let Err(_x) = gpio.write(&data) {
                        defmt::error!("GPIO write failed: {}", data[0]);
                    }
                }
                trigger_pendsv();
            }
        })
    }

    fn start(&self) {
        self.inner.lock(|inner| {
            inner.started = true;

            if inner.task_count > 0 {
                inner.current = 0;
                if let Some(task) = &mut inner.tasks[0] {
                    task.state = TaskState::Running;
                }
            }
        })
    }

    fn get_tick(&self) -> u64 {
        self.inner.lock(|inner| inner.tick_count)
    }

    fn switch(&self, old_sp: *mut u32) -> *mut u32 {
        self.inner.lock(|inner| {
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
                core::ptr::null_mut()
            }
        })
    }
}

// static mut CURR_SCHEDULER_INNER: SchedulerInner = SchedulerInner::new();

static CURR_SCHEDULER: Scheduler = Scheduler::new();

pub fn scheduler() -> &'static dyn interface::Scheduler {
    &CURR_SCHEDULER
}

/// Called from PendSV handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_switch(old_sp: *mut u32) -> *mut u32 {
    scheduler().switch(old_sp)
}
