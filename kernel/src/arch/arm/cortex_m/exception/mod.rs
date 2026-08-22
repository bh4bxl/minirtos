use cortex_m::peripheral::{SCB, scb::SystemHandler};

mod fault;
mod pendsv;
pub(super) mod svc;
mod systick;

/// Initialize Cortex-M exception priorities used by the kernel.
///
/// PendSV should normally run at the lowest priority so context switching
/// happens after higher-priority interrupts complete.
pub fn init(mut scb: SCB) {
    //
    // Priority setup can be added here when needed.
    //
    unsafe {
        scb.set_priority(SystemHandler::SVCall, 0x40);
        scb.set_priority(SystemHandler::SysTick, 0x80);
        scb.set_priority(SystemHandler::PendSV, 0xFF);
    }
}

/// Request a PendSV exception.
///
/// Used by the scheduler to defer a context switch until exception return.
#[inline]
pub(crate) fn pend_pendsv() {
    cortex_m::peripheral::SCB::set_pendsv();

    // Ensure the write is observed before continuing.
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

/// Return the currently active exception number.
///
/// `0` means Thread mode.
#[inline]
pub fn exception_number() -> usize {
    let ipsr: u32;

    unsafe {
        core::arch::asm!(
            "mrs {0}, IPSR",
            out(reg) ipsr,
            options(nomem, nostack, preserves_flags),
        );
    }

    (ipsr & 0x1ff) as usize
}

/// Return true when currently executing in Thread mode.
#[inline]
pub fn in_thread_mode() -> bool {
    exception_number() == 0
}
