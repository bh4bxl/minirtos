use core::sync::atomic::{AtomicBool, Ordering};

use rp235x_hal::{self as hal, Watchdog, clocks, pac};

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt::Rp235xIrqManger,
    drivers::{
        self, gpio::interface::Gpio, lcd::interface::Lcd, spi::interface::SpiBus,
        uart::interface::Uart,
    },
    gui,
    sys::{
        board, console,
        device_driver::{self, DevError},
        interrupt::{interface::IrqManager, register_irq_manager},
    },
};

static IRQ_MANAGER: Rp235xIrqManger = Rp235xIrqManger::new();

static GPIO: drivers::gpio::rp235x_gpio::Rp235xGpio = drivers::gpio::rp235x_gpio::Rp235xGpio::new();

fn gpio_config() -> Result<(), DevError> {
    use crate::drivers::gpio::{Direction, Function, Pull};

    // Uart0 pins;
    GPIO.pin_config(0, Function::UART, Pull::None, None);
    GPIO.pin_config(1, Function::UART, Pull::Up, None);

    // Spi1 pins
    GPIO.pin_config(10, Function::SPI, Pull::None, None);
    GPIO.pin_config(11, Function::SPI, Pull::None, None);

    // Lcd pins
    // dc
    GPIO.pin_config(8, Function::SIO, Pull::None, Some(Direction::Output));
    // cs
    GPIO.pin_config(9, Function::SIO, Pull::None, Some(Direction::Output));
    // rst
    GPIO.pin_config(12, Function::SIO, Pull::None, Some(Direction::Output));
    // backlight
    GPIO.pin_config(13, Function::SIO, Pull::None, Some(Direction::Output));

    // Test Pins
    GPIO.pin_config(19, Function::SIO, Pull::None, Some(Direction::Output));
    GPIO.pin_config(21, Function::SIO, Pull::None, Some(Direction::Output));
    // GPIO.pin_config(27, Function::SIO, Pull::Up, Some(Direction::Input));
    // GPIO.enable_irq(&Pin(27), GpioIrqTrigger::EdgeLow, 0);

    Ok(())
}

fn gpio_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &GPIO,
        Some(gpio_config),
        Some(rp235x_pac::Interrupt::IO_IRQ_BANK0),
        device_driver::DeviceType::Gpio,
    );
    device_driver::driver_manager().register(descriptor)
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
    device_driver::driver_manager().register(descriptor)
}

static SPI1: drivers::spi::rp235x_pl022_spi::Pl022Spi =
    drivers::spi::rp235x_pl022_spi::Pl022Spi::new(drivers::spi::rp235x_pl022_spi::SpiId::SPI1);

fn spi_config() -> Result<(), DevError> {
    let mut config = drivers::spi::SpiConfig::default();
    config.baudrate = 25_000_000;
    SPI1.config(&config);

    SPI1.enable_dma(drivers::spi::DmaDir::Tx, true);

    Ok(())
}

fn spi_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &SPI1,
        Some(spi_config),
        None,
        device_driver::DeviceType::Spi,
    );
    device_driver::driver_manager().register(descriptor)
}

static LCD_WIDTH: usize = 240;
static LCD_HEIGHT: usize = 135;

static LCD: drivers::lcd::st7789vw::St7789vwLcd<LCD_WIDTH, LCD_HEIGHT> =
    drivers::lcd::st7789vw::St7789vwLcd::<LCD_WIDTH, LCD_HEIGHT>::new(&SPI1, &GPIO, 8, 12, 9);

fn lcd_config() -> Result<(), DevError> {
    let config = drivers::lcd::LcdConfig::default();
    LCD.config(&config)?;

    LCD.display_on()?;

    GPIO.set_level(&drivers::gpio::Pin(13), drivers::gpio::Level::High);

    gui::register_lcd_flush(&LCD);

    Ok(())
}

fn lcd_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &LCD,
        Some(lcd_config),
        None,
        device_driver::DeviceType::Lcd,
    );
    device_driver::driver_manager().register(descriptor)
}

static KEYBOARD: drivers::input::ws114_gpio_kb::Ws114GpioKeyboard =
    drivers::input::ws114_gpio_kb::Ws114GpioKeyboard::new(&GPIO);

fn keyboard_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &KEYBOARD,
        None,
        None,
        device_driver::DeviceType::Input,
    );
    device_driver::driver_manager().register(descriptor)
}

static CYW43: drivers::wlan::cyw43::Cyw43 = drivers::wlan::cyw43::Cyw43::new(&GPIO, 29, 24, 25, 23);
// static CYW43: drivers::wlan::cyw43::Cyw43 = drivers::wlan::cyw43::Cyw43::new(&GPIO, 19, 21, 22, 23);

fn cyw43_register(pio0: pac::PIO0, resets: &mut pac::RESETS) -> Result<(), DevError> {
    CYW43.init_hw(pio0, resets)?;

    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &CYW43,
        None,
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

    gpio_register()?;

    uart_register()?;

    spi_register()?;

    lcd_register()?;

    keyboard_register()?;

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
