use cortex_m::{interrupt::InterruptNumber, peripheral::NVIC, register::primask};

use crate::arch::Arch;

#[derive(Clone, Copy)]
struct IrqNumber(u16);

unsafe impl InterruptNumber for IrqNumber {
    #[inline]
    fn number(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct IrqState {
    primask: u32,
}

#[inline]
pub fn disable() -> IrqState {
    // Save the exact previous PRIMASK state.
    let state = IrqState {
        primask: primask::read_raw(),
    };

    cortex_m::interrupt::disable();

    state
}

#[inline]
pub fn restore(state: IrqState) {
    unsafe {
        primask::write_raw(state.primask);
    }
}

#[inline]
pub fn enabled() -> bool {
    primask::read_raw() == 0
}

#[inline]
pub fn enable_irq(irq: u32) {
    debug_assert!((irq as usize) < super::CortexM::MAX_IRQS);

    unsafe {
        NVIC::unmask(IrqNumber(irq as u16));
    }
}

#[inline]
pub fn disable_irq(irq: u32) {
    debug_assert!((irq as usize) < super::CortexM::MAX_IRQS);

    NVIC::mask(IrqNumber(irq as u16));
}

#[inline]
pub fn clear_pending_irq(irq: u32) {
    debug_assert!((irq as usize) < super::CortexM::MAX_IRQS);

    NVIC::unpend(IrqNumber(irq as u16));
}
