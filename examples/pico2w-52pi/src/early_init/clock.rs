use rp235x_hal as hal;
use rp235x_pac as pac;

use minirtos_drivers::DevError;

pub fn init_clocks(
    watchdog: pac::WATCHDOG,
    xosc: pac::XOSC,
    clock: pac::CLOCKS,
    pll_sys: pac::PLL_SYS,
    pll_usb: pac::PLL_USB,
    mut resets: pac::RESETS,
) -> Result<(hal::clocks::ClocksManager, pac::RESETS), DevError> {
    defmt::info!("Initializing clock");

    let mut watchdog = hal::Watchdog::new(watchdog);

    let clocks = hal::clocks::init_clocks_and_plls(
        12_000_000,
        xosc,
        clock,
        pll_sys,
        pll_usb,
        &mut resets,
        &mut watchdog,
    )
    .map_err(|_| DevError::InitFailed)?;

    Ok((clocks, resets))
}
