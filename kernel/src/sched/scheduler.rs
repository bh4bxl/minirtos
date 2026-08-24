use alloc::vec::Vec;

use crate::{
    SysError, arch,
    memory::{self, StackRegion},
    synchronization::{CriticalSection, CriticalSectionLock, critical_section, interface::Lock},
    task::{PendingIpc, Priority, Privilege, TaskControl, TaskEntry, TaskId, TaskInfo, TaskState},
};

use super::{
    WaitTaskResult,
    idle_task::{IDLE_STACK_SIZE, IDLE_TASK_ID, idle_task_entry},
};

pub const MAX_TASKS: usize = 16;

struct SchedulerInner {
    tasks: [Option<TaskControl>; MAX_TASKS],

    current: usize,
    task_count: usize,

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
        // Find the highest priority among Ready tasks.
        // Lower numeric value means higher priority.
        let best_priority = self
            .tasks
            .iter()
            .flatten()
            .filter(|task| task.state == TaskState::Ready)
            .map(|task| task.priority)
            .min()
            .unwrap_or(Priority(255));

        // Pick a task after the current task to provide round-robin
        // scheduling between tasks with equal priority.
        let start = (self.current + 1) % MAX_TASKS;

        for offset in 0..MAX_TASKS {
            let index = (start + offset) % MAX_TASKS;

            if let Some(task) = &self.tasks[index] {
                if task.state == TaskState::Ready && task.priority == best_priority {
                    return index;
                }
            }
        }

        IDLE_TASK_ID
    }
}

pub(super) struct Scheduler {
    inner: CriticalSectionLock<SchedulerInner>,
}

impl Scheduler {
    pub(super) const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(SchedulerInner::new()),
        }
    }

    pub(super) fn reap_one(&self, cs: &CriticalSection) -> Option<StackRegion> {
        self.inner.lock(cs, |inner| {
            for index in 1..MAX_TASKS {
                // Never reap the currently running task.
                if index == inner.current {
                    continue;
                }

                let should_reap = inner.tasks[index]
                    .as_ref()
                    .is_some_and(|task| task.state == TaskState::Terminated);

                if !should_reap {
                    continue;
                }

                let task = inner.tasks[index]
                    .take()
                    .expect("terminated task must exist");

                debug_assert_eq!(task.owned_mutex_count, 0);

                inner.task_count -= 1;

                return Some(task.into_stack());
            }

            None
        })
    }
}

impl super::interface::Scheduler for Scheduler {
    fn init(&self, cs: &CriticalSection) {
        // Allocate outside the scheduler inner lock.
        //
        // The pool itself has its own synchronization.
        let need_idle = self
            .inner
            .lock(cs, |inner| inner.tasks[IDLE_TASK_ID].is_none());

        if !need_idle {
            return;
        }

        let stack = memory::STACK_POOL
            .lock(|pool| pool.alloc(IDLE_STACK_SIZE))
            .expect("failed to allocate idle task stack");

        let idle = TaskControl::new(
            idle_task_entry,
            core::ptr::null_mut(),
            stack,
            Priority(255),
            Privilege::Privileged,
            "idle",
        )
        .with_time_slice(1);

        self.inner.lock(cs, |inner| {
            if inner.tasks[IDLE_TASK_ID].is_none() {
                inner.tasks[IDLE_TASK_ID] = Some(idle);
                inner.task_count += 1;
            } else {
                // This normally cannot happen during single-threaded
                // scheduler initialization.
                //
                // If init semantics become concurrent later, return
                // the allocated stack to the pool here.
                panic!("idle task initialized concurrently");
            }
        });
    }

    fn add_task(
        &self,
        cs: &CriticalSection,
        entry: TaskEntry,
        arg: *mut (),
        stack: StackRegion,
        priority: Priority,
        privilege: Privilege,
        name: &'static str,
    ) -> Result<TaskId, (SysError, StackRegion)> {
        self.inner.lock(cs, |inner| {
            let Some(slot) = inner.tasks.iter_mut().find(|slot| slot.is_none()) else {
                return Err((SysError::NoResource, stack));
            };

            let task = TaskControl::new(entry, arg, stack, priority, privilege, name);

            let task_id = task.id;

            *slot = Some(task);

            inner.task_count += 1;

            Ok(task_id)
        })
    }

    fn current_task_id(&self, cs: &CriticalSection) -> TaskId {
        self.inner.lock(cs, |inner| {
            inner.tasks[inner.current]
                .as_ref()
                .expect("current task must exist")
                .id
        })
    }

    fn set_current_task_status(&self, cs: &CriticalSection, state: TaskState) {
        self.inner.lock(cs, |inner| {
            if let Some(task) = &mut inner.tasks[inner.current] {
                task.state = state;
            }
        });
    }

    fn current_task_sleep_ticks(&self, cs: &CriticalSection, ticks: u64) {
        self.inner.lock(cs, |inner| {
            let wake_tick = inner.tick_count.saturating_add(ticks);

            let task = inner.tasks[inner.current]
                .as_mut()
                .expect("current task missing");

            task.state = TaskState::Sleeping;
            task.wake_tick = wake_tick;
        });
    }

    fn exit_current_task(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            let current_index = inner.current;

            if current_index == IDLE_TASK_ID {
                panic!("idle task must not exit");
            }

            let waiter = {
                let task = inner.tasks[current_index]
                    .as_mut()
                    .expect("current task missing");

                if task.owned_mutex_count != 0 {
                    panic!(
                        "task {} exited while holding {} mutexes",
                        task.name, task.owned_mutex_count
                    );
                }

                task.state = TaskState::Terminated;
                task.remaining_slice = 0;
                task.wake_tick = 0;

                task.waiter.take()
            };

            if let Some(waiter_id) = waiter {
                if let Some(waiter_task) = inner
                    .tasks
                    .iter_mut()
                    .flatten()
                    .find(|task| task.id == waiter_id)
                {
                    if waiter_task.state == TaskState::Blocked {
                        waiter_task.state = TaskState::Ready;
                    }
                }
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
                if task.id != id {
                    continue;
                }

                if task.state == TaskState::Blocked {
                    task.state = TaskState::Ready;
                }

                break;
            }
        });
    }

    fn set_pending_ipc(
        &self,
        cs: &CriticalSection,
        id: TaskId,
        pending: PendingIpc,
    ) -> Result<(), SysError> {
        self.inner.lock(cs, |inner| {
            let task = inner
                .tasks
                .iter_mut()
                .flatten()
                .find(|task| task.id == id)
                .ok_or(SysError::NotFound)?;

            task.pending_ipc = pending;

            Ok(())
        })
    }

    fn take_pending_ipc(&self, cs: &CriticalSection, id: TaskId) -> Result<PendingIpc, SysError> {
        self.inner.lock(cs, |inner| {
            let task = inner
                .tasks
                .iter_mut()
                .flatten()
                .find(|task| task.id == id)
                .ok_or(SysError::NotFound)?;

            Ok(core::mem::replace(&mut task.pending_ipc, PendingIpc::None))
        })
    }

    fn wait_task(&self, cs: &CriticalSection, target: TaskId) -> Result<WaitTaskResult, SysError> {
        self.inner.lock(cs, |inner| {
            let current_index = inner.current;

            let current_id = inner.tasks[current_index]
                .as_ref()
                .expect("current task missing")
                .id;

            if current_id == target {
                return Err(SysError::InvalidArgument);
            }

            let target_index = inner
                .tasks
                .iter()
                .position(|slot| slot.as_ref().is_some_and(|task| task.id == target))
                .ok_or(SysError::NotFound)?;

            let target_task = inner.tasks[target_index]
                .as_ref()
                .expect("target task missing");

            if target_task.state == TaskState::Terminated {
                return Ok(WaitTaskResult::Terminated);
            }

            if let Some(waiter) = target_task.waiter {
                if waiter != current_id {
                    return Err(SysError::Busy);
                }
            }

            inner.tasks[target_index]
                .as_mut()
                .expect("target task missing")
                .waiter = Some(current_id);

            inner.tasks[current_index]
                .as_mut()
                .expect("current task missing")
                .state = TaskState::Blocked;

            Ok(WaitTaskResult::Blocked)
        })
    }

    fn reap_task(&self, cs: &CriticalSection, target: TaskId) -> Result<StackRegion, SysError> {
        self.inner.lock(cs, |inner| {
            let target_index = inner
                .tasks
                .iter()
                .position(|slot| slot.as_ref().is_some_and(|task| task.id == target))
                .ok_or(SysError::NotFound)?;

            if target_index == IDLE_TASK_ID || target_index == inner.current {
                return Err(SysError::Busy);
            }

            let task = inner.tasks[target_index]
                .as_ref()
                .expect("target task missing");

            if task.state != TaskState::Terminated {
                return Err(SysError::Busy);
            }

            let task = inner.tasks[target_index]
                .take()
                .expect("target task missing");

            inner.task_count -= 1;

            Ok(task.into_stack())
        })
    }

    fn update_tick(&self, cs: &CriticalSection) {
        let need_switch = self.inner.lock(cs, |inner| {
            inner.tick_count += 1;
            let now = inner.tick_count;

            let mut task_woken = false;

            // Wake sleeping tasks.
            for task in inner.tasks.iter_mut().flatten() {
                if task.state == TaskState::Sleeping && task.wake_tick <= now {
                    task.state = TaskState::Ready;
                    task_woken = true;
                }
            }

            if inner.task_count == 0 || !inner.started {
                return false;
            }

            let current = inner.current;

            let Some(task) = inner.tasks[current].as_mut() else {
                return true;
            };

            // Current task is no longer runnable.
            if task.state != TaskState::Running {
                return true;
            }

            task.remaining_slice = task.remaining_slice.saturating_sub(1);

            let slice_expired = task.remaining_slice == 0;

            if slice_expired {
                task.remaining_slice = task.time_slice;
            }

            task_woken || slice_expired
        });

        if need_switch {
            arch::request_context_switch();
        }
    }

    fn start(&self, cs: &CriticalSection) -> arch::Context {
        self.inner.lock(cs, |inner| {
            inner.started = true;

            inner.current = inner.next_task();

            let task = inner.tasks[inner.current]
                .as_mut()
                .expect("idle task must exist");

            task.state = TaskState::Running;

            arch::prepare_context_switch(&task.context);

            task.context
        })
    }

    fn start_unchecked(&self) -> arch::Context {
        unsafe {
            self.inner.lock_unchecked(|inner| {
                inner.started = true;

                inner.current = inner.next_task();

                let task = inner.tasks[inner.current]
                    .as_mut()
                    .expect("first task must exist");

                task.state = TaskState::Running;

                arch::prepare_context_switch(&task.context);

                task.context
            })
        }
    }

    fn mutex_acquired(&self, cs: &CriticalSection, id: TaskId) {
        self.inner.lock(cs, |inner| {
            if let Some(task) = inner.tasks.iter_mut().flatten().find(|task| task.id == id) {
                task.owned_mutex_count += 1;
            }
        });
    }

    fn mutex_released(&self, cs: &CriticalSection, id: TaskId) {
        self.inner.lock(cs, |inner| {
            if let Some(task) = inner.tasks.iter_mut().flatten().find(|task| task.id == id) {
                assert!(
                    task.owned_mutex_count > 0,
                    "mutex ownership count underflow"
                );

                task.owned_mutex_count -= 1;
            }
        });
    }

    fn get_tick(&self, cs: &CriticalSection) -> u64 {
        self.inner.lock(cs, |inner| inner.tick_count)
    }

    fn tasks(&self) -> Vec<TaskInfo> {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                inner
                    .tasks
                    .iter()
                    .filter_map(|task| {
                        task.as_ref().map(|task| TaskInfo {
                            id: task.id,
                            name: task.name,
                            state: task.state,
                            priority: task.priority,
                            privilege: task.privilege,
                            stack_used: task.stack_used_bytes(),
                            stack_total: task.stack_total_bytes(),
                        })
                    })
                    .collect()
            })
        })
    }

    unsafe fn switch(&self, old_sp: usize) -> arch::Context {
        unsafe {
            self.inner.lock_unchecked(|inner| {
                //
                // Save outgoing task context.
                //
                if let Some(task) = inner.tasks[inner.current].as_mut() {
                    arch::set_stack_pointer(&mut task.context, old_sp);

                    assert!(
                        task.stack_sp_in_range(),
                        "task {} SP out of range: sp={:#x} stack={:#x}..{:#x}",
                        task.name,
                        arch::stack_pointer(&task.context),
                        task.stack.start(),
                        task.stack.end(),
                    );

                    task.check_stack_guard();

                    if task.state == TaskState::Running {
                        task.state = TaskState::Ready;
                    }
                }

                //
                // Select next runnable task.
                //
                inner.current = inner.next_task();

                if inner.tasks[inner.current].is_none() {
                    inner.current = IDLE_TASK_ID;
                }

                let task = inner.tasks[inner.current]
                    .as_mut()
                    .expect("idle task must exist");

                task.state = TaskState::Running;

                if task.remaining_slice == 0 {
                    task.remaining_slice = task.time_slice;
                }

                //
                // Architecture-specific state that must be
                // restored alongside the stack pointer.
                //
                arch::prepare_context_switch(&task.context);

                task.context
            })
        }
    }
}
