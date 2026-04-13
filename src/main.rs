#![no_std]
#![no_main]

use cortex_m::asm;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use crate::{bsp::board_init, sys::console};
use rp235x_hal as hal;

mod bsp;
mod drivers;
mod sys;

#[hal::entry]
fn main() -> ! {
    info!("MINI RTOS");

    match board_init() {
        Err(e) => error!("Error: {}", e),
        Ok(()) => info!("Board {} initialized.", sys::board::board().board_name()),
    }

    console::console().write_str("miniRTOS\r\n");
    console::console().clear_rx();
    loop {
        asm::wfi();
    }
}
