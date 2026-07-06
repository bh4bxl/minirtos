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
/// Create a thread
pub fn thread_create(
    thread_entry: TaskEntry,
    arg: *mut (),
    stack: &'static mut [u32],
    priority: Priority,
    name: &'static str,
) -> Result<TaskId, SysError> {
    critical_section(|cs| {
        scheduler::scheduler().add_task(cs, thread_entry, arg, stack, priority, name)
    })
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
