use crate::sys::{
    SysError,
    arch::arm_cortex_m::trigger_pendsv,
    scheduler,
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

pub fn task_exit() -> ! {
    critical_section(|cs| scheduler::scheduler().exit_current_task(cs));

    trigger_pendsv();
    cortex_m::asm::isb();

    loop {
        cortex_m::asm::wfi();
    }
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
