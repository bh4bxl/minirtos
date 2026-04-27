use core::sync::atomic::{AtomicBool, Ordering};

use rp235x_hal::{self as hal, Watchdog, clocks, pac};

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt::Rp235xIrqManger,
    drivers::{
        self, gpio::interface::Gpio, lcd::interface::Lcd, spi::interface::SpiBus,
        uart::interface::Uart,
    },
    sys::{
        board, console,
        device_driver::{self, DevError},
        interrupt::register_irq_manager,
    },
};

static IRQ_MANAGER: Rp235xIrqManger = Rp235xIrqManger::new();

static GPIO: drivers::gpio::rp235x_gpio::Rp235xGpio = drivers::gpio::rp235x_gpio::Rp235xGpio::new();

fn gpio_config() -> Result<(), DevError> {
    // Uart0 pins;
    GPIO.pin_config(
        0,
        drivers::gpio::Function::UART,
        drivers::gpio::Pull::None,
        None,
    );
    GPIO.pin_config(
        1,
        drivers::gpio::Function::UART,
        drivers::gpio::Pull::Up,
        None,
    );

    // Spi1 pins
    GPIO.pin_config(
        10,
        drivers::gpio::Function::SPI,
        drivers::gpio::Pull::None,
        None,
    );
    GPIO.pin_config(
        11,
        drivers::gpio::Function::SPI,
        drivers::gpio::Pull::None,
        None,
    );

    // Lcd pins
    // dc
    GPIO.pin_config(
        8,
        drivers::gpio::Function::SIO,
        drivers::gpio::Pull::None,
        Some(drivers::gpio::Direction::Output),
    );
    // cs
    GPIO.pin_config(
        9,
        drivers::gpio::Function::SIO,
        drivers::gpio::Pull::None,
        Some(drivers::gpio::Direction::Output),
    );
    // rst
    GPIO.pin_config(
        12,
        drivers::gpio::Function::SIO,
        drivers::gpio::Pull::None,
        Some(drivers::gpio::Direction::Output),
    );
    // backlight
    GPIO.pin_config(
        13,
        drivers::gpio::Function::SIO,
        drivers::gpio::Pull::None,
        Some(drivers::gpio::Direction::Output),
    );

    // Test Pins
    GPIO.pin_config(
        19,
        drivers::gpio::Function::SIO,
        drivers::gpio::Pull::None,
        Some(drivers::gpio::Direction::Output),
    );
    GPIO.pin_config(
        21,
        drivers::gpio::Function::SIO,
        drivers::gpio::Pull::None,
        Some(drivers::gpio::Direction::Output),
    );

    Ok(())
}

fn gpio_register() -> Result<(), DevError> {
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

fn uart_config() -> Result<(), DevError> {
    let config = drivers::uart::Config::default();
    UART0.config(&config);

    console::register_console(&UART0);

    Ok(())
}

fn uart_register() -> Result<(), DevError> {
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

fn spi_config() -> Result<(), DevError> {
    let mut config = drivers::spi::SpiConfig::default();
    config.baudrate = 20_000_000;
    SPI1.config(&config);

    Ok(())
}

fn spi_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &SPI1,
        Some(spi_config),
        None,
        device_driver::DeviceType::Spi,
    );
    device_driver::driver_manager().register(descriptor);

    Ok(())
}

static LCD: drivers::lcd::st7789vw::St7789vwLcd =
    drivers::lcd::st7789vw::St7789vwLcd::new(&SPI1, &GPIO, 8, 12, 9);

fn lcd_config() -> Result<(), DevError> {
    let config = drivers::lcd::LcdConfig::default();
    LCD.config(&config)?;

    LCD.display_on()?;

    GPIO.set_level(&drivers::gpio::Pin(13), drivers::gpio::Level::High);

    Ok(())
}

fn lcd_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &LCD,
        Some(lcd_config),
        None,
        device_driver::DeviceType::Lcd,
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

fn init_clock() -> Result<(), DevError> {
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

pub fn board_init() -> Result<(), DevError> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err(DevError::DevAlreadyInit);
    }

    init_clock()?;

    register_irq_manager(&IRQ_MANAGER);

    gpio_register()?;

    uart_register()?;

    spi_register()?;

    lcd_register()?;

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
