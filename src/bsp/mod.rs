pub mod boards;
pub mod mcu;

use defmt::info;
pub use rp235x_pac as pac;

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt::systick_init,
    println,
    sys::{board, device_driver, interrupt::irq_manager},
};

pub fn board_init() -> Result<(), &'static str> {
    boards::pico2w::board_init()?;

    unsafe {
        device_driver::driver_manager().init_drivers();
    }

    info!("Registered drivers ({}):", board::board().board_name());
    device_driver::driver_manager().enumerate();

    info!("Registered interrupts:");
    irq_manager().enumerate();

    let cp = cortex_m::Peripherals::take().unwrap();

    systick_init(cp.SYST, 150_000_000, 1000);

    println!("Board {} initialized.", board::board().board_name());

    Ok(())
}
