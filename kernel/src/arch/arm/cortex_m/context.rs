use core::ptr;

use crate::task::{Privilege, TaskEntry, TaskExit};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// Saved process stack pointer.
    psp: usize,

    /// CONTROL register value restored when the thread runs.
    control: u32,
}

/// Cortex-M hardware exception stack frame.
///
/// This frame is automatically restored by exception return.
#[repr(C)]
struct ExceptionFrame {
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
}

/// Registers that Cortex-M does not automatically stack.
///
/// PendSV saves/restores these registers manually.
#[repr(C)]
struct SoftwareFrame {
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
}

const XPSR_THUMB: u32 = 1 << 24;

/// EXC_RETURN:
/// return to Thread mode, use PSP, no floating-point state.
pub const EXC_RETURN_THREAD_PSP: u32 = 0xFFFF_FFFD;

pub fn init(
    stack_top: *mut u8,
    entry: TaskEntry,
    arg: *mut (),
    exit: TaskExit,
    privilege: Privilege,
) -> Context {
    let mut sp = align_down(stack_top as usize, 8);

    /*
     * Initial stack layout:
     *
     * high address
     * +------------------+
     * | xPSR             |
     * | PC               |
     * | LR               |
     * | R12              |
     * | R3               |
     * | R2               |
     * | R1               |
     * | R0 = arg         |
     * +------------------+
     * | R11              |
     * | R10              |
     * | R9               |
     * | R8               |
     * | R7               |
     * | R6               |
     * | R5               |
     * | R4               |
     * +------------------+
     * low address <- PSP
     */

    sp -= size_of::<ExceptionFrame>();

    let exception_frame = sp as *mut ExceptionFrame;

    unsafe {
        ptr::write(
            exception_frame,
            ExceptionFrame {
                r0: arg as u32,
                r1: 0,
                r2: 0,
                r3: 0,
                r12: 0,
                lr: exit as usize as u32,
                pc: entry as usize as u32,
                xpsr: XPSR_THUMB,
            },
        );
    }

    sp -= size_of::<SoftwareFrame>();

    let software_frame = sp as *mut SoftwareFrame;

    unsafe {
        ptr::write(
            software_frame,
            SoftwareFrame {
                r4: 0,
                r5: 0,
                r6: 0,
                r7: 0,
                r8: 0,
                r9: 0,
                r10: 0,
                r11: 0,
            },
        );
    }

    Context {
        psp: sp,
        control: match privilege {
            Privilege::Privileged => 0b10,
            Privilege::Unprivileged => 0b11,
        },
    }
}

#[inline]
pub fn stack_pointer(context: &Context) -> usize {
    context.psp
}

#[inline]
pub fn current_psp() -> usize {
    cortex_m::register::psp::read() as usize
}

#[inline]
pub fn set_stack_pointer(context: &mut Context, sp: usize) {
    context.psp = sp;
}

#[inline]
pub fn prepare_switch(context: &Context) {
    unsafe {
        NEXT_TASK_CONTROL = context.control;
    }
}

#[unsafe(no_mangle)]
static mut NEXT_TASK_CONTROL: u32 = 0b10;

#[inline]
pub fn is_privileged() -> bool {
    cortex_m::register::control::read().npriv().is_privileged()
}

#[inline]
const fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

/// Switch from main context to the first task.
pub(super) fn restore_first(context: &Context) -> ! {
    let sp = context.psp;
    let control = context.control;

    unsafe {
        core::arch::asm!(
            // Restore software-saved registers.
            "ldmia r0!, {{r4-r11}}",

            // PSP now points to hardware exception frame.
            "msr psp, r0",

            // Thread mode:
            // SPSEL = 1
            // nPRIV = task privilege
            "msr CONTROL, r1",
            "isb",

            // Exception return:
            // Thread mode + PSP
            "ldr lr, =0xFFFFFFFD",
            "bx lr",

            in("r0") sp,
            in("r1") control,

            options(noreturn),
        );
    }
}
