use core::arch::asm;

/// Switch from main context to the first task.
pub unsafe fn start_first_task() -> ! {
    unsafe {
        asm!("svc 0", options(noreturn));
    }
}
