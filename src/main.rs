#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use crate::{bsp::board_init, sys::arch::arm_cortex_m::start_first_task};
use rp235x_hal as hal;

mod apps;
mod bsp;
mod drivers;
mod gui;
mod net;
mod services;
mod sys;

#[hal::entry]
fn main() -> ! {
    defmt::info!("MINI RTOS");

    match board_init() {
        Err(e) => defmt::error!("Error: {:?}", e as u16),
        Ok(()) => defmt::info!("Board {} initialized.", sys::board::board().board_name()),
    }

    m_info!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    sys::kernel_init().unwrap();

    apps::shell::start_shell().unwrap();

    apps::hmi::start_hmi().unwrap();

    gui::input::input_manager::InputManager::start().unwrap();

    #[cfg(feature = "cyw43")]
    apps::wlan::start_wlan().unwrap();

    // apps::net_test::start_net_test_apps().unwrap();

    unsafe {
        start_first_task();
    }
}
