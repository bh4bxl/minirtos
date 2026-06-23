#![cfg(feature = "pico2w-picocalc")]

use crate::{
    drivers::{
        self,
        gpio::interface::Gpio,
        lcd::{DisplayRoation, interface::Lcd},
        spi::interface::SpiBus,
        uart::interface::Uart,
    },
    gui,
    sys::{
        console,
        device_driver::{self, DevError},
    },
};

fn gpio_config() -> Result<(), DevError> {
    use crate::drivers::gpio::{Function, Pull};

    // Uart0 pins;
    super::GPIO.pin_config(0, Function::UART, Pull::None, None);
    super::GPIO.pin_config(1, Function::UART, Pull::Up, None);

    // Spi1 pins
    super::GPIO.pin_config(10, Function::SPI, Pull::None, None);
    super::GPIO.pin_config(11, Function::SPI, Pull::None, None);

    // I2c1 pins
    super::GPIO.pin_config(6, Function::I2C, Pull::Up, None);
    super::GPIO.pin_config(7, Function::I2C, Pull::Up, None);

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

static I2C1: drivers::i2c::rp235x_i2c::Rp235xI2c =
    drivers::i2c::rp235x_i2c::Rp235xI2c::new(drivers::i2c::rp235x_i2c::I2cId::I2C1);

fn i2c_config() -> Result<(), DevError> {
    let mut config = drivers::i2c::I2cConfig::default();

    config.baudrate = 100_000;

    I2C1.config(&config);

    Ok(())
}

fn i2c_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &I2C1,
        Some(i2c_config),
        None,
        device_driver::DeviceType::I2c,
    );
    device_driver::driver_manager().register(descriptor)
}

static LCD_WIDTH: usize = 320;
static LCD_HEIGHT: usize = 320;

static LCD: drivers::lcd::ili9488::Ili9488Lcd<LCD_WIDTH, LCD_HEIGHT> =
    drivers::lcd::ili9488::Ili9488Lcd::<LCD_WIDTH, LCD_HEIGHT>::new(
        &SPI1,
        &super::GPIO,
        14,
        15,
        13,
    );

fn lcd_config() -> Result<(), DevError> {
    let mut config = drivers::lcd::LcdConfig::default();

    config.ration = DisplayRoation::Roation90;

    config.x_offset = 0;
    config.y_offset = 0;
    LCD.config(&config)?;

    LCD.display_on()?;

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

static KB_ADDR: u8 = 0x1f;
static KEYBOARD: drivers::input::picocalc_kb::PicocalcKeyboard =
    drivers::input::picocalc_kb::PicocalcKeyboard::new(&I2C1, KB_ADDR);

fn keyboard_config() -> Result<(), DevError> {
    gui::input::register_keyboard(&KEYBOARD);

    Ok(())
}

fn keyboard_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &KEYBOARD,
        Some(keyboard_config),
        None,
        device_driver::DeviceType::Input,
    );
    device_driver::driver_manager().register(descriptor)
}

pub fn mb_board_init() -> Result<(), DevError> {
    gpio_register()?;

    uart_register()?;

    spi_register()?;

    i2c_register()?;

    lcd_register()?;

    keyboard_register()?;

    Ok(())
}
