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

        // Set Thread-mode privilege and stack selection
        ldr r1, =NEXT_TASK_CONTROL
        ldr r1, [r1]
        msr control, r1
        isb

        // Exception return:
        // CPU will automatically restore r0-r3, r12, lr, pc, xpsr
        // from PSP, then continue running the selected task in Thread mode.
        bx lr
        "
    )
}
