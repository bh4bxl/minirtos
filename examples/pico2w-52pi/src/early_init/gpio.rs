use minirtos_drivers::DevError;

pub(super) fn init() -> Result<(), DevError> {
    let pads = unsafe { &*rp235x_pac::PADS_BANK0::ptr() };
    let io = unsafe { &*rp235x_pac::IO_BANK0::ptr() };

    // GPIO0 = UART0 TX
    pads.gpio(0).modify(|_, w| {
        w.iso()
            .clear_bit()
            .od()
            .clear_bit()
            .ie()
            .set_bit()
            .pue()
            .clear_bit()
            .pde()
            .clear_bit()
    });

    io.gpio(0).gpio_ctrl().modify(|_, w| w.funcsel().uart());

    // GPIO1 = UART0 RX
    pads.gpio(1).modify(|_, w| {
        w.iso()
            .clear_bit()
            .od()
            .clear_bit()
            .ie()
            .set_bit()
            .pue()
            .set_bit()
            .pde()
            .clear_bit()
    });

    io.gpio(1).gpio_ctrl().modify(|_, w| w.funcsel().uart());

    Ok(())
}
