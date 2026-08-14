use core::sync::atomic::{AtomicBool, Ordering};

use rp_binary_info as binary_info;
use rp235x_hal as hal;
use rp235x_pac as pac;

use minirtos_drivers::DevError;

mod clock;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [binary_info::EntryAddr; 5] = [
    binary_info::rp_cargo_bin_name!(),
    binary_info::rp_cargo_version!(),
    binary_info::rp_program_description!(c"RP2350 miniRTOS"),
    binary_info::rp_cargo_homepage_url!(),
    binary_info::rp_program_build_attribute!(),
];

pub fn early_init() -> Result<(), DevError> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);

    if INIT_DONE.load(Ordering::Relaxed) {
        return Err(DevError::AlreadyInitialized);
    }

    let pac = pac::Peripherals::take().unwrap();

    let (_, _) = clock::init_clocks(
        pac.WATCHDOG,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        pac.RESETS,
    )?;

    INIT_DONE.store(true, Ordering::Release);

    Ok(())
}
