pub mod boards;
pub mod mcu;

pub use rp235x_pac as pac;

use crate::{
    m_info,
    sys::{
        arch::arm_cortex_m::{init_exception_priority, systick_init},
        board,
        device_driver::{self, DevError},
        interrupt::irq_manager,
    },
};

pub fn board_init() -> Result<(), DevError> {
    let cp = cortex_m::Peripherals::take().unwrap();
    systick_init(cp.SYST, 150_000_000, 1000);

    init_exception_priority(cp.SCB);

    boards::pico2w::board_init()?;

    unsafe {
        device_driver::driver_manager().init_drivers();
    }

    m_info!("Registered drivers ({}):", board::board().board_name());
    device_driver::driver_manager().enumerate();

    m_info!("Registered interrupts:");
    irq_manager().enumerate();

    m_info!("Board {} initialized.", board::board().board_name());

    Ok(())
}
