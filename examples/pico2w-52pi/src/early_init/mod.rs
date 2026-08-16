use core::sync::atomic::{AtomicBool, Ordering};

use minirtos_kernel::KernelConfig;
use rp_binary_info as binary_info;
use rp235x_hal::{self as hal, Clock};
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

pub fn early_init(config: &mut KernelConfig) -> Result<(), DevError> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);

    if INIT_DONE.load(Ordering::Relaxed) {
        return Err(DevError::AlreadyInitialized);
    }

    let pac = pac::Peripherals::take().unwrap();

    let (clocks, _) = clock::init_clocks(
        pac.WATCHDOG,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        pac.RESETS,
    )?;

    defmt::info!(
        "         CPU clock: {} MHz",
        &clocks.system_clock.freq().to_MHz()
    );
    defmt::info!(
        "  Peripheral clock: {} MHz",
        &clocks.peripheral_clock.freq().to_MHz()
    );
    defmt::info!(
        "         ADC clock: {} MHz",
        &clocks.adc_clock.freq().to_MHz()
    );
    defmt::info!(
        "         USB clock: {} MHz",
        &clocks.usb_clock.freq().to_MHz()
    );
    defmt::info!(
        "   Reference clock: {} MHz",
        &clocks.reference_clock.freq().to_MHz()
    );

    config.core_clock_hz = clocks.system_clock.freq().to_Hz();

    INIT_DONE.store(true, Ordering::Release);

    Ok(())
}
