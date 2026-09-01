use core::sync::atomic::{AtomicBool, Ordering};

use crate::{
    arch,
    memory::STACK_POOL,
    synchronization::{critical_section, interface::Lock},
};

mod idle_task;
mod scheduler;

use minirtos_abi::SysError;
use scheduler::Scheduler;

enum WaitTaskResult {
    Blocked,
    Terminated,
}

pub mod interface {
    use alloc::vec::Vec;

    use crate::{
        SysError, arch,
        ipc::PendingIpc,
        memory::StackRegion,
        synchronization::CriticalSection,
        task::{Priority, Privilege, TaskEntry, TaskId, TaskInfo, TaskState},
    };

    pub trait Scheduler {
        fn init(&self, cs: &CriticalSection) -> Result<(), SysError>;

        fn add_task(
            &self,
            cs: &CriticalSection,
            entry: TaskEntry,
            arg: *mut (),
            stack: StackRegion,
            priority: Priority,
            privilege: Privilege,
            name: &'static str,
        ) -> Result<TaskId, (SysError, StackRegion)>;

        fn current_task_id(&self, cs: &CriticalSection) -> TaskId;

        fn set_current_task_status(&self, cs: &CriticalSection, state: TaskState);

        fn current_task_sleep_ticks(&self, cs: &CriticalSection, ms: u64);

        fn exit_current_task(&self, cs: &CriticalSection);

        fn block_current_task(&self, cs: &CriticalSection);

        fn wake_task(&self, cs: &CriticalSection, id: TaskId);

        fn set_pending_ipc(
            &self,
            cs: &CriticalSection,
            id: TaskId,
            pending: PendingIpc,
        ) -> Result<(), SysError>;

        fn take_pending_ipc(
            &self,
            cs: &CriticalSection,
            id: TaskId,
        ) -> Result<PendingIpc, SysError>;

        fn set_syscall_result(
            &self,
            cs: &CriticalSection,
            id: TaskId,
            res: i32,
        ) -> Result<(), SysError>;

        fn wait_task(
            &self,
            cs: &CriticalSection,
            target: TaskId,
        ) -> Result<super::WaitTaskResult, SysError>;

        fn reap_task(&self, cs: &CriticalSection, target: TaskId) -> Result<StackRegion, SysError>;

        fn update_tick(&self, cs: &CriticalSection);

        fn start(&self, cs: &CriticalSection) -> arch::Context;

        fn start_unchecked(&self) -> arch::Context;

        fn mutex_acquired(&self, cs: &CriticalSection, id: TaskId);

        fn mutex_released(&self, cs: &CriticalSection, id: TaskId);

        fn add_task_rw_region(
            &self,
            cs: &CriticalSection,
            id: TaskId,
            base: usize,
            size: usize,
        ) -> Result<(), SysError>;

        fn remove_task_region(
            &self,
            cs: &CriticalSection,
            id: TaskId,
            base: usize,
            size: usize,
        ) -> Result<(), SysError>;

        fn get_tick(&self, cs: &CriticalSection) -> u64;

        fn tasks(&self) -> Vec<TaskInfo>;

        /// Called by the architecture context-switch handler.
        ///
        /// Interrupts are already disabled and the scheduler lock
        /// must therefore be accessed through lock_unchecked().
        ///
        /// `old_sp` is the stack pointer saved by the architecture.
        unsafe fn switch(&self, old_sp: usize) -> arch::Context;
    }
}

static CURR_SCHEDULER: Scheduler = Scheduler::new();

pub(crate) fn scheduler() -> &'static dyn interface::Scheduler {
    &CURR_SCHEDULER
}

static SCHEDULER_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() -> Result<(), SysError> {
    if SCHEDULER_INITIALIZED.swap(true, Ordering::Acquire) {
        return Ok(());
    }

    critical_section(|cs| scheduler().init(cs))?;

    SCHEDULER_INITIALIZED.store(true, Ordering::Release);

    Ok(())
}

/// Called from the architecture context-switch handler.
///
/// Keep this ABI temporarily compatible with the existing Cortex-M
/// PendSV handler:
///
/// input  = outgoing PSP
/// output = incoming PSP
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_switch(old_sp: *mut u32) -> *mut u32 {
    let context = unsafe { scheduler().switch(old_sp as usize) };

    arch::stack_pointer(&context) as *mut u32
}

fn reap_terminated_tasks() {
    loop {
        let stack = critical_section(|cs| CURR_SCHEDULER.reap_one(cs));

        let Some(stack) = stack else {
            break;
        };

        STACK_POOL.lock(|pool| {
            pool.free(stack);
        });
    }
}

//
// Helpers
//

pub(crate) fn terminate_current_task() {
    critical_section(|cs| {
        scheduler().exit_current_task(cs);
    });
}

pub(crate) fn exit_current_task() -> ! {
    terminate_current_task();

    arch::request_context_switch();

    loop {
        arch::wait_for_interrupt();
    }
}

pub(crate) fn sleep_current_task(ms: u32) {
    let ticks = crate::timer::ms_to_ticks(ms);

    critical_section(|cs| {
        scheduler().current_task_sleep_ticks(cs, ticks);
    });
}
