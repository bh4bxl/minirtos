#![no_std]
#![no_main]

mod early_init;

use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_probe as _;

#[entry]
fn main() -> ! {
    defmt::info!(
        "miniRTOS {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );

    match early_init::early_init() {
        Err(e) => {
            defmt::error!("Error: {:?}", e as u16);
            panic!("early init failed");
        }
        Ok(()) => defmt::info!("Board {} early initialized.", env!("CARGO_PKG_NAME"),),
    }

    minirtos_kernel::init();

    loop {
        cortex_m::asm::nop();
    }
}
