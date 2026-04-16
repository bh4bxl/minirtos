pub mod boards;
pub mod mcu;

use defmt::info;
pub use rp235x_pac as pac;

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt,
    sys::{board, console, driver_manager, interrupt::irq_manager},
};

pub fn board_init() -> Result<(), &'static str> {
    boards::pico2w::board_init()?;

    unsafe {
        driver_manager::driver_manager().init_drivers();
    }

    info!("Registered drivers ({}):", board::board().board_name());
    driver_manager::driver_manager().enumerate();

    info!("Registered interrupts:");
    irq_manager().enumerate();

    console::console().write_str("Board ");
    console::console().write_str(board::board().board_name());
    console::console().write_str(" initialized.\r\n");

    Ok(())
}
