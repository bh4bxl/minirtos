use core::sync::atomic::{AtomicBool, Ordering};

use rp235x_hal::{self as hal, Watchdog, clocks, pac};

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt::Rp235xIrqManger,
    drivers, net,
    sys::{
        board,
        device_driver::{self, DevError},
        interrupt::{interface::IrqManager, register_irq_manager},
    },
};

pub mod bd52pi;
pub mod picocalc;
pub mod ws_lcd114;

#[cfg(feature = "pico2w-ws-lcd114")]
use ws_lcd114::mb_board_init;

#[cfg(feature = "pico2w-52pi")]
use bd52pi::mb_board_init;

#[cfg(feature = "pico2w-picocalc")]
use picocalc::mb_board_init;

static IRQ_MANAGER: Rp235xIrqManger = Rp235xIrqManger::new();

static GPIO: drivers::gpio::rp235x_gpio::Rp235xGpio = drivers::gpio::rp235x_gpio::Rp235xGpio::new();

static CYW43: drivers::wlan::cyw43::Cyw43 = drivers::wlan::cyw43::Cyw43::new(&GPIO, 29, 24, 25, 23);

fn cyw43_config() -> Result<(), DevError> {
    net::register_wlan(&CYW43);
    Ok(())
}

fn cyw43_register(pio0: pac::PIO0, resets: &mut pac::RESETS) -> Result<(), DevError> {
    CYW43.init_hw(pio0, resets)?;

    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &CYW43,
        Some(cyw43_config),
        None,
        device_driver::DeviceType::Wlan,
    );
    device_driver::driver_manager().register(descriptor)
}

pub struct Pico2wBoard;

impl board::interface::Info for Pico2wBoard {
    fn board_name(&self) -> &'static str {
        "Raspberry Pico 2W"
    }
}

impl board::interface::All for Pico2wBoard {}

static PICO2W_BOARD: Pico2wBoard = Pico2wBoard {};

fn init_clock(
    watchdog: pac::WATCHDOG,
    xosc: pac::XOSC,
    clock: pac::CLOCKS,
    pll_sys: pac::PLL_SYS,
    pll_usb: pac::PLL_USB,
    mut resets: pac::RESETS,
) -> Result<pac::RESETS, DevError> {
    defmt::info!("Initializing clock");

    // clocks
    let mut watchdog = Watchdog::new(watchdog);

    let _clocks = clocks::init_clocks_and_plls(
        12_000_000,
        xosc,
        clock,
        pll_sys,
        pll_usb,
        &mut resets,
        &mut watchdog,
    );

    Ok(resets)
}

fn init_dma() -> Result<(), DevError> {
    defmt::info!("Initializing DMA");

    let resets = unsafe { &*pac::RESETS::ptr() };
    resets.reset().modify(|_, w| w.dma().clear_bit());
    while resets.reset_done().read().dma().bit_is_clear() {}

    Ok(())
}

pub fn board_init() -> Result<(), DevError> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err(DevError::DevAlreadyInit);
    }

    let pac = pac::Peripherals::take().unwrap();

    let mut resets = init_clock(
        pac.WATCHDOG,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        pac.RESETS,
    )?;

    init_dma()?;

    register_irq_manager(&IRQ_MANAGER);

    IRQ_MANAGER.enable(false);

    mb_board_init()?;

    cyw43_register(pac.PIO0, &mut resets)?;

    board::register_board(&PICO2W_BOARD);

    INIT_DONE.store(true, Ordering::Release);

    IRQ_MANAGER.enable(true);

    Ok(())
}

/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    rp235x_hal::binary_info::rp_cargo_bin_name!(),
    rp235x_hal::binary_info::rp_cargo_version!(),
    rp235x_hal::binary_info::rp_program_description!(c"RP2350 miniRTOS"),
    rp235x_hal::binary_info::rp_cargo_homepage_url!(),
    rp235x_hal::binary_info::rp_program_build_attribute!(),
];
