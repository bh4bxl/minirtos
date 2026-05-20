#![allow(dead_code)]

use crate::{
    drivers::{
        self, gpio::interface::Gpio, lcd::interface::Lcd, spi::interface::SpiBus,
        uart::interface::Uart,
    },
    gui,
    sys::{
        console,
        device_driver::{self, DevError},
    },
};

fn gpio_config() -> Result<(), DevError> {
    use crate::drivers::gpio::{Direction, Function, Pull};

    // Uart0 pins;
    super::GPIO.pin_config(0, Function::UART, Pull::None, None);
    super::GPIO.pin_config(1, Function::UART, Pull::Up, None);

    // Spi1 pins
    super::GPIO.pin_config(10, Function::SPI, Pull::None, None);
    super::GPIO.pin_config(11, Function::SPI, Pull::None, None);

    // Lcd pins
    // dc
    super::GPIO.pin_config(8, Function::SIO, Pull::None, Some(Direction::Output));
    // cs
    super::GPIO.pin_config(9, Function::SIO, Pull::None, Some(Direction::Output));
    // rst
    super::GPIO.pin_config(12, Function::SIO, Pull::None, Some(Direction::Output));
    // backlight
    super::GPIO.pin_config(13, Function::SIO, Pull::None, Some(Direction::Output));

    // Test Pins
    super::GPIO.pin_config(19, Function::SIO, Pull::None, Some(Direction::Output));
    super::GPIO.pin_config(21, Function::SIO, Pull::None, Some(Direction::Output));
    // GPIO.pin_config(27, Function::SIO, Pull::Up, Some(Direction::Input));
    // GPIO.enable_irq(&Pin(27), GpioIrqTrigger::EdgeLow, 0);

    Ok(())
}

fn gpio_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &super::GPIO,
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
    drivers::lcd::st7789vw::St7789vwLcd::<LCD_WIDTH, LCD_HEIGHT>::new(
        &SPI1,
        &super::GPIO,
        8,
        12,
        9,
    );

fn lcd_config() -> Result<(), DevError> {
    let config = drivers::lcd::LcdConfig::default();
    LCD.config(&config)?;

    LCD.display_on()?;

    super::GPIO.set_level(&drivers::gpio::Pin(13), drivers::gpio::Level::High);

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
    drivers::input::ws114_gpio_kb::Ws114GpioKeyboard::new(&super::GPIO);

fn keyboard_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &KEYBOARD,
        None,
        None,
        device_driver::DeviceType::Input,
    );
    device_driver::driver_manager().register(descriptor)
}

pub fn mb_board_init() -> Result<(), DevError> {
    gpio_register()?;

    uart_register()?;

    spi_register()?;

    lcd_register()?;

    keyboard_register()?;

    Ok(())
}
