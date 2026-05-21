#[cfg(feature = "pico2w-52pi")]
use crate::{
    drivers::gpio::{Level, Pin, interface::Gpio},
    gui,
    sys::device_driver::DevError,
};
use crate::{
    drivers::{
        self,
        lcd::{DisplayRoation, interface::Lcd},
        spi::interface::SpiBus,
        uart::interface::Uart,
    },
    sys::{console, device_driver},
};

#[cfg(feature = "pico2w-52pi")]

fn gpio_config() -> Result<(), DevError> {
    use crate::drivers::gpio::{Direction, Function, Pull};

    // Uart0 pins;
    super::GPIO.pin_config(0, Function::UART, Pull::None, None);
    super::GPIO.pin_config(1, Function::UART, Pull::Up, None);

    // Spi0 pins
    super::GPIO.pin_config(2, Function::SPI, Pull::None, None);
    super::GPIO.pin_config(3, Function::SPI, Pull::None, None);

    // I2c0 pins
    super::GPIO.pin_config(8, Function::I2C, Pull::Up, None);
    super::GPIO.pin_config(9, Function::I2C, Pull::Up, None);

    // Lcd pins
    // dc
    super::GPIO.pin_config(6, Function::SIO, Pull::None, Some(Direction::Output));
    // cs
    super::GPIO.pin_config(5, Function::SIO, Pull::None, Some(Direction::Output));
    // rst
    super::GPIO.pin_config(7, Function::SIO, Pull::None, Some(Direction::Output));

    // beepere
    super::GPIO.pin_config(13, Function::SIO, Pull::None, Some(Direction::Output));
    super::GPIO.set_level(&Pin(13), Level::Low);

    // Led1
    super::GPIO.pin_config(16, Function::SIO, Pull::None, Some(Direction::Output));
    super::GPIO.set_level(&Pin(16), Level::High);
    // Led2
    super::GPIO.pin_config(17, Function::SIO, Pull::None, Some(Direction::Output));
    super::GPIO.set_level(&Pin(17), Level::High);
    // RGB Led
    super::GPIO.pin_config(12, Function::SIO, Pull::None, Some(Direction::Output));
    super::GPIO.set_level(&Pin(12), Level::High);
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

static SPI0: drivers::spi::rp235x_pl022_spi::Pl022Spi =
    drivers::spi::rp235x_pl022_spi::Pl022Spi::new(drivers::spi::rp235x_pl022_spi::SpiId::SPI0);

fn spi_config() -> Result<(), DevError> {
    let mut config = drivers::spi::SpiConfig::default();
    config.baudrate = 25_000_000;
    SPI0.config(&config);

    SPI0.enable_dma(drivers::spi::DmaDir::Tx, true);

    Ok(())
}

fn spi_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &SPI0,
        Some(spi_config),
        None,
        device_driver::DeviceType::Spi,
    );
    device_driver::driver_manager().register(descriptor)
}

static I2C0: drivers::i2c::rp235x_i2c::Rp235xI2c =
    drivers::i2c::rp235x_i2c::Rp235xI2c::new(drivers::i2c::rp235x_i2c::I2cId::I2C0);

fn i2c_config() -> Result<(), DevError> {
    let config = drivers::i2c::I2cConfig::default();

    I2C0.config(&config);

    Ok(())
}

fn i2c_register() -> Result<(), DevError> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &I2C0,
        Some(i2c_config),
        None,
        device_driver::DeviceType::I2c,
    );
    device_driver::driver_manager().register(descriptor)
}

static LCD_WIDTH: usize = 320;
static LCD_HEIGHT: usize = 480;

static LCD: drivers::lcd::st7796su1::St7796su1Lcd<LCD_WIDTH, LCD_HEIGHT> =
    drivers::lcd::st7796su1::St7796su1Lcd::<LCD_WIDTH, LCD_HEIGHT>::new(
        &SPI0,
        &super::GPIO,
        6,
        7,
        5,
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

pub fn mb_board_init() -> Result<(), DevError> {
    gpio_register()?;

    uart_register()?;

    spi_register()?;

    i2c_register()?;

    lcd_register()?;

    Ok(())
}
