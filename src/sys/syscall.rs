use super::{
    SysError,
    arch::arm_cortex_m::trigger_pendsv,
    scheduler::{self, WaitTaskResult},
    synchronization::{critical_section, interface::Mutex},
    task::{Priority, TaskEntry, TaskId},
};

/// Voluntarily yield the CPU to the next ready task.
#[inline(always)]
pub fn yield_now() {
    trigger_pendsv();
    cortex_m::asm::isb();
}

// ===== Clock =====
pub fn get_tick() -> u64 {
    critical_section(|cs| scheduler::scheduler().get_tick(cs))
}

/// Sleep for `ms` milliseconds.
pub fn sleep_ms(ms: u32) {
    critical_section(|cs| {
        scheduler::scheduler().current_task_sleep(cs, ms);
    });
    yield_now();
}

#[allow(dead_code)]
/// Spawn a task
pub fn task_spawn(
    task_entry: TaskEntry,
    arg: *mut (),
    stack_words: usize,
    priority: Priority,
    name: &'static str,
) -> Result<TaskId, SysError> {
    let stack = super::task::STACK_POOL.lock(|pool| pool.alloc_words(stack_words))?;

    match critical_section(|cs| {
        scheduler::scheduler().add_task(cs, task_entry, arg, stack, priority, name)
    }) {
        Ok(task_id) => Ok(task_id),

        Err((error, stack)) => {
            super::task::STACK_POOL.lock(|pool| {
                pool.free_words(stack);
            });

            Err(error)
        }
    }
}

pub fn task_wait(task_id: TaskId) -> Result<(), SysError> {
    let wait_result = critical_section(|cs| scheduler::scheduler().wait_task(cs, task_id))?;

    if matches!(wait_result, WaitTaskResult::Blocked) {
        super::arch::arm_cortex_m::trigger_pendsv();
    }

    let stack = critical_section(|cs| scheduler::scheduler().reap_task(cs, task_id))?;

    super::task::STACK_POOL.lock(|pool| {
        pool.free_words(stack);
    });

    Ok(())
}

#[allow(dead_code)]
/// End current task
pub fn task_exit() -> ! {
    super::task::exit_current_task();
}

pub fn stack_pool_total() -> usize {
    super::task::STACK_POOL.lock(|inner| inner.total())
}

pub fn stack_pool_used() -> usize {
    super::task::STACK_POOL.lock(|inner| inner.used())
}

pub fn stack_pool_free() -> usize {
    super::task::STACK_POOL.lock(|inner| inner.free())
}
