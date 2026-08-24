use crate::{
    sched,
    sys::{SyscallId, SyscallResult},
};

#[unsafe(export_name = "SVCall")]
#[unsafe(naked)]
pub unsafe extern "C" fn svc_handler() -> ! {
    core::arch::naked_asm!(
        "
        // Determine which stack contains the exception frame.
        //
        // EXC_RETURN bit 2:
        //   0 = MSP
        //   1 = PSP
        tst lr, #4
        ite eq
        mrseq r0, msp
        mrsne r0, psp

        // r0 = exception stack frame
        //
        // Decode the SVC immediate.
        // frame[6] = stacked PC
        ldr r1, [r0, #24]

        // SVC is a 16-bit Thumb instruction.
        // stacked PC points to the instruction after SVC.
        ldrb r1, [r1, #-2]

        // SVC 0 is reserved for starting the scheduler.
        cmp r1, #0
        beq 1f

        // Normal syscall.
        //
        // Preserve EXC_RETURN in LR while calling Rust.
        // r3 is pushed as padding to keep MSP 8-byte aligned.
        push {{r3, lr}}

        // r0 = exception frame
        bl svc_dispatch

        pop {{r3, lr}}

        // Return from SVC.
        //
        // If PendSV was pended by yield/exit, Cortex-M can
        // tail-chain directly into PendSV.
        bx lr

    1:
        // Does not return.
        b svc_start_first_task
        "
    )
}

#[unsafe(no_mangle)]
unsafe extern "C" fn svc_dispatch(frame: *mut u32) {
    unsafe {
        // Hardware exception frame:
        //
        // frame[0] = r0
        // frame[1] = r1
        // frame[2] = r2
        // frame[3] = r3
        // frame[4] = r12
        // frame[5] = lr
        // frame[6] = pc
        // frame[7] = xPSR

        let pc = *frame.add(6) as *const u8;

        // SVC is a 16-bit Thumb instruction.
        let svc = *pc.sub(2);
        let Ok(svc) = SyscallId::try_from(svc) else {
            return;
        };

        let op = *frame.add(0);
        let args = [*frame.add(1), *frame.add(2), *frame.add(3)];

        let res = match svc {
            SyscallId::Task => crate::sys::task_dispatch(op, &args),

            SyscallId::Sync => crate::sys::sync_dispatch(op, &args),

            SyscallId::Ipc => crate::sys::ipc_dispatch(op, &args),

            _ => {
                // Unknown SVC.
                // For now just return to the caller.
                SyscallResult::None
            }
        };
        match res {
            SyscallResult::U32(r) => *frame.add(0) = r,
            SyscallResult::U64(r) => {
                *frame.add(0) = r as u32;
                *frame.add(1) = (r >> 32) as u32;
            }
            SyscallResult::Error(err) => {
                *frame.add(0) = err as i32 as u32;
            }
            SyscallResult::None => {}
        }
    }
}

#[unsafe(no_mangle)]
unsafe fn svc_start_first_task() -> ! {
    let context = sched::scheduler().start_unchecked();

    super::super::context::restore_first(&context)
}

pub fn start_first_task() -> ! {
    unsafe {
        core::arch::asm!("svc 0", options(noreturn));
    }
}
