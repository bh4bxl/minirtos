use cortex_m::peripheral::{SYST, syst::SystClkSource};

pub(super) fn init_timer(mut syst: SYST, core_clock_hz: u32, tick_hz: u32) {
    assert!(core_clock_hz > 0 && tick_hz > 0);

    let reload = core_clock_hz / tick_hz - 1;

    // SysTick reload is 24-bit.
    assert!(reload <= 0x00ff_ffff);

    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(reload);
    syst.clear_current();

    syst.enable_interrupt();
    syst.enable_counter();
}
