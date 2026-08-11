use core::arch::asm;

use cortex_m::peripheral::scb::SystemHandler;

use crate::sys::{console, scheduler, synchronization::critical_section, syscall::Syscall};

pub fn systick_init(mut syst: cortex_m::peripheral::SYST, cpu_hz: u32, tick_hz: u32) {
    let reload = cpu_hz / tick_hz - 1;

    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(reload);
    syst.clear_current();

    syst.enable_interrupt();
    syst.enable_counter();
}

pub fn init_exception_priority(mut scb: cortex_m::peripheral::SCB) {
    unsafe {
        scb.set_priority(SystemHandler::PendSV, 0xFF);
        scb.set_priority(SystemHandler::SysTick, 0x80);
        scb.set_priority(SystemHandler::SVCall, 0x40);
    }
}

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
    critical_section(|cs| {
        scheduler::scheduler().update_tick(cs);
    });
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
        let Ok(svc) = Syscall::try_from(svc) else {
            return;
        };

        match svc {
            Syscall::Yield => {
                trigger_pendsv();
            }
            Syscall::Exit => {
                super::super::task::terminate_current_task();
                trigger_pendsv();
            }
            Syscall::Sleep => {
                let ms = *frame.add(0);
                super::super::task::sleep_current_task(ms);
                trigger_pendsv();
            }
            Syscall::Write => {
                let ptr = *frame.add(0) as *const u8;
                let len: usize = *frame.add(1) as usize;

                if len == 0 || ptr.is_null() {
                    return;
                }

                let data = core::slice::from_raw_parts(ptr, len);

                if let Ok(s) = core::str::from_utf8(data) {
                    console::console().write_str(s);
                }
            }
            Syscall::TryReadChar => {
                let ret = match console::console().try_read_char() {
                    Some(c) => c as u32,
                    None => u32::MAX,
                };

                *frame.add(0) = ret;
            }
            Syscall::GetTick => {
                let tick = scheduler::get_sys_tick();

                *frame.add(0) = tick as u32;
                *frame.add(1) = (tick >> 32) as u32;
            }

            _ => {
                // Unknown SVC.
                // For now just return to the caller.
            }
        }
    }
}

#[unsafe(no_mangle)]
unsafe fn svc_start_first_task() -> ! {
    unsafe {
        let sched = scheduler::scheduler();
        let (sp, control) = critical_section(|cs| {
            sched.start(cs);
            (sched.current_task_sp(cs), sched.current_task_control(cs))
        });

        core::arch::asm!(
            // Restore r4-r11 from task stack
            "ldmia {sp}!, {{r4-r11}}",

            // PSP now points to the hardware exception frame.
            "msr psp, {sp}",

            // Configure Thread mode:
            //   SPSEL = 1
            //   nPRIV = task privilege
            "msr CONTROL, {control}",
            "isb",

            // Return to Thread mode using PSP.
            "ldr lr, =0xFFFFFFFD",
            "bx lr",

            sp = in(reg) sp,
            control = in(reg) control,

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

/// HardFault handler
#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &cortex_m_rt::ExceptionFrame) -> ! {
    let psp: u32;
    let msp: u32;
    let control: u32;

    unsafe {
        core::arch::asm!("mrs {}, psp", out(reg) psp);
        core::arch::asm!("mrs {}, msp", out(reg) msp);
        core::arch::asm!("mrs {}, control", out(reg) control);
    }

    crate::m_error!("HardFault!");
    crate::m_error!(
        "PC={:08x} LR={:08x} xPSR={:08x}",
        ef.pc(),
        ef.lr(),
        ef.xpsr()
    );
    crate::m_error!(
        "PSP=0x{:08x} MSP=0x{:08x} CONTROL=0x{:08x}",
        psp,
        msp,
        control
    );

    loop {
        cortex_m::asm::bkpt();
    }
}
