use crate::arch;

pub(super) const IDLE_TASK_ID: usize = 0;
pub(super) const IDLE_STACK_SIZE: usize = 8192;

pub(super) extern "C" fn idle_task_entry(_arg: *mut ()) {
    loop {
        super::reap_terminated_tasks();

        arch::wait_for_interrupt();
    }
}
