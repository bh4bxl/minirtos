use core::sync::atomic::{AtomicBool, Ordering};

use rp235x_hal::{self as hal, Watchdog, clocks, pac};

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt::Rp235xIrqManger,
    drivers::{self, gpio::Gpio, spi::interface::SpiBus, uart::interface::Uart},
    sys::{board, console, device_driver, interrupt::register_irq_manager},
};

static IRQ_MANAGER: Rp235xIrqManger = Rp235xIrqManger::new();

static GPIO: drivers::gpio::rp235x_gpio::Rp235xGpio = drivers::gpio::rp235x_gpio::Rp235xGpio::new();

fn gpio_config() -> Result<(), &'static str> {
    // Uart0 pins;
    let (uart0_tx, uart0_rx) = (drivers::gpio::Pin(0), drivers::gpio::Pin(1));
    GPIO.eable(&uart0_tx, true);
    GPIO.eable(&uart0_rx, true);
    GPIO.set_pull(&uart0_tx, drivers::gpio::Pull::None);
    GPIO.set_function(&uart0_tx, drivers::gpio::Function::UART);
    GPIO.set_pull(&uart0_rx, drivers::gpio::Pull::Up);
    GPIO.set_function(&uart0_rx, drivers::gpio::Function::UART);

    // Spi1 pins
    let (spi1_sck, spi1_mosi) = (drivers::gpio::Pin(10), drivers::gpio::Pin(11));
    GPIO.eable(&spi1_sck, true);
    GPIO.eable(&spi1_mosi, true);
    GPIO.set_pull(&spi1_sck, drivers::gpio::Pull::None);
    GPIO.set_pull(&spi1_mosi, drivers::gpio::Pull::None);
    GPIO.set_function(&spi1_sck, drivers::gpio::Function::SPI);
    GPIO.set_function(&spi1_mosi, drivers::gpio::Function::SPI);

    // Test Pin
    let test_pin1 = drivers::gpio::Pin(19);
    GPIO.eable(&test_pin1, true);
    GPIO.set_function(&test_pin1, drivers::gpio::Function::SIO);
    GPIO.set_pull(&test_pin1, drivers::gpio::Pull::None);
    GPIO.set_direction(&test_pin1, drivers::gpio::Direction::Output, true);
    GPIO.set_level(&test_pin1, drivers::gpio::Level::Low);
    let test_pin2 = drivers::gpio::Pin(21);
    GPIO.eable(&test_pin2, true);
    GPIO.set_function(&test_pin2, drivers::gpio::Function::SIO);
    GPIO.set_pull(&test_pin2, drivers::gpio::Pull::None);
    GPIO.set_direction(&test_pin2, drivers::gpio::Direction::Output, true);
    GPIO.set_level(&test_pin2, drivers::gpio::Level::Low);

    Ok(())
}

fn gpio_register() -> Result<(), &'static str> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &GPIO,
        Some(gpio_config),
        None,
        device_driver::DeviceType::Gpio,
    );
    device_driver::driver_manager().register(descriptor);

    Ok(())
}

static UART0: drivers::uart::rp235x_pl011_uart::Pl011Uart =
    drivers::uart::rp235x_pl011_uart::Pl011Uart::new(
        drivers::uart::rp235x_pl011_uart::UartId::UART0,
    );

fn uart_config() -> Result<(), &'static str> {
    let config = drivers::uart::Config::default();
    UART0.config(&config);

    console::register_console(&UART0);

    Ok(())
}

fn uart_register() -> Result<(), &'static str> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &UART0,
        Some(uart_config),
        Some(rp235x_pac::Interrupt::UART0_IRQ),
        device_driver::DeviceType::Uart,
    );
    device_driver::driver_manager().register(descriptor);

    Ok(())
}

static SPI1: drivers::spi::rp235x_pl022_spi::Pl022Spi =
    drivers::spi::rp235x_pl022_spi::Pl022Spi::new(drivers::spi::rp235x_pl022_spi::SpiId::SPI1);

fn spi_config() -> Result<(), &'static str> {
    let config = drivers::spi::SpiConfig::default();
    SPI1.config(&config);

    Ok(())
}

fn spi_register() -> Result<(), &'static str> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &SPI1,
        Some(spi_config),
        None,
        device_driver::DeviceType::Spi,
    );
    device_driver::driver_manager().register(descriptor);

    Ok(())
}

pub struct Pico2wBoard;

impl board::interface::Info for Pico2wBoard {
    fn board_name(&self) -> &'static str {
        "Raspberry Pico 2W"
    }
}

impl board::interface::All for Pico2wBoard {}

static PICO2W_BOARD: Pico2wBoard = Pico2wBoard {};

fn init_clock() -> Result<(), &'static str> {
    let mut pac = pac::Peripherals::take().unwrap();

    // clocks
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let _clocks = clocks::init_clocks_and_plls(
        12_000_000,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    );
    Ok(())
}

pub fn board_init() -> Result<(), &'static str> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err("Init already done");
    }

    init_clock()?;

    register_irq_manager(&IRQ_MANAGER);

    gpio_register()?;

    uart_register()?;

    spi_register()?;

    board::register_board(&PICO2W_BOARD);

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
