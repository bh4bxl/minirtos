use cortex_m::register::primask;

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
