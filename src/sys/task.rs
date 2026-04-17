pub type TaskEntry = fn() -> !;

/// Called if a task entry function ever returns.
fn task_exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
pub struct TaskControlBlock {
    pub sp: *mut u32,
}

impl TaskControlBlock {
    pub const fn new() -> Self {
        Self {
            sp: core::ptr::null_mut(),
        }
    }
}

/// Initialize the initial stack frame for a Cortex-M task.
///
/// Stack layout after initialization:
/// - software-saved registers: r4-r11
/// - hardware-stacked registers:
///   r0, r1, r2, r3, r12, lr, pc, xpsr
///
/// Returns the initial SP value to restore this task.
pub unsafe fn init_task_stack(
    stack_bottom: *mut u32,
    stack_words: usize,
    entry: TaskEntry,
) -> *mut u32 {
    let mut sp = unsafe { stack_bottom.add(stack_words) };

    // 8-byte align stack pointer
    sp = ((sp as usize) & !0x7) as *mut u32;

    // Hardware-stacked frame (exception return will pop these)
    sp = unsafe { sp.sub(1) };
    unsafe { *sp = 0x0100_0000 }; // xPSR, Thumb bit set

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = entry as usize as u32 }; // PC

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = task_exit as *const () as usize as u32 }; // LR

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = 0 }; // R12

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = 0 }; // R3

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = 0 }; // R2

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = 0 }; // R1

    sp = unsafe { sp.sub(1) };
    unsafe { *sp = 0 }; // R0

    // Software-saved frame: r4-r11
    for _ in 0..8 {
        sp = unsafe { sp.sub(1) };
        unsafe { *sp = 0 };
    }

    sp
}
