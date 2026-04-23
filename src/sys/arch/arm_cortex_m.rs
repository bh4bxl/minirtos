use core::arch::asm;

use crate::sys::scheduler;

/// Trigger a context switch by pending PendSV.
#[inline(always)]
pub fn trigger_pendsv() {
    // Write PENDSVSET bit in ICSR
    unsafe {
        core::ptr::write_volatile(0xE000_ED04 as *mut u32, 1 << 28);
    }
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

#[cortex_m_rt::exception]
fn SysTick() {
    scheduler::scheduler().update_tick();
}

#[unsafe(export_name = "PendSV")]
#[unsafe(naked)]
pub unsafe extern "C" fn pendsv_handler() -> ! {
    core::arch::naked_asm!(
        "
        // r0 = current PSP (current task's process stack pointer)
        mrs r0, psp

        // Save software-saved registers r4-r11 onto current task stack.
        // Exception entry has already stacked:
        // r0-r3, r12, lr, pc, xpsr
        // So PendSV only needs to save r4-r11.
        stmdb r0!, {{r4-r11}}

        // IMPORTANT:
        // On exception entry, LR contains EXC_RETURN.
        // 'bl scheduler_switch' will overwrite LR with a normal return address.
        // So we must save/restore LR, otherwise 'bx lr' at the end will not
        // perform exception return correctly.
        //
        // Push an extra register together with LR to keep MSP 8-byte aligned.
        push {{r3, lr}}

        // Call scheduler_switch(old_sp)
        //   input : r0 = old task sp
        //   output: r0 = new task sp
        bl scheduler_switch

        // Restore EXC_RETURN into LR
        pop {{r3, lr}}

        // Restore software-saved registers of next task from its stack
        ldmia r0!, {{r4-r11}}

        // Update PSP to the remaining hardware-stacked frame of next task
        msr psp, r0

        // Exception return:
        // CPU will automatically restore r0-r3, r12, lr, pc, xpsr
        // from PSP, then continue running the selected task in Thread mode.
        bx lr
        "
    )
}

#[cortex_m_rt::exception]
unsafe fn SVCall() {
    unsafe {
        scheduler::scheduler().start();
        let sp = scheduler::scheduler().current_task_sp();
        core::arch::asm!(
            // Restore r4-r11 from task stack
            "ldmia {sp}!, {{r4-r11}}",
            // PSP = remaining hardware frame
            "msr psp, {sp}",
            // Thread mode use PSP
            "movs r0, #2",
            "msr CONTROL, r0",
            "isb",
            // Exception return to thread mode using PSP
            "ldr lr, =0xFFFFFFFD",
            "bx lr",
            sp = in(reg) sp,
            options(noreturn)
        );
    }
}

/// Switch from main context to the first task.
pub unsafe fn start_first_task() -> ! {
    unsafe {
        asm!("svc 0", options(noreturn));
    }
}
