use super::task::TaskControlBlock;

static mut CURRENT_TCB: *mut TaskControlBlock = core::ptr::null_mut();

/// # Safety
/// Caller must ensure exclusive access to scheduler state.
pub unsafe fn set_current_task(tcb: *mut TaskControlBlock) {
    unsafe {
        CURRENT_TCB = tcb;
    }
}

pub unsafe fn current_task() -> *mut TaskControlBlock {
    unsafe { CURRENT_TCB }
}
