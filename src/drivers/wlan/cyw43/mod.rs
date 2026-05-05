use rp235x_pac as pac;

use crate::{
    drivers::{delay_ms, gpio},
    sys::{
        device_driver, interrupt,
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

pub mod cyw43_bus;
pub mod firmware;
pub mod pio_ctrl;
pub mod pio_spi;

struct Cyw43Inner {
    gpio: &'static dyn gpio::interface::Gpio,
    bus: cyw43_bus::Cyw43Bus,

    wl_dio: gpio::Pin,
    wl_cs: gpio::Pin,
    wl_on: gpio::Pin,

    bus_is_up: bool,
}

impl Cyw43Inner {
    const fn new(
        gpio: &'static dyn gpio::interface::Gpio,
        wl_clk: usize,
        wl_dio: usize,
        wl_cs: usize,
        wl_on: usize,
    ) -> Self {
        let pio_spi = pio_spi::PioSpi::new(gpio, wl_clk, wl_dio, wl_cs);
        Self {
            gpio,
            bus: cyw43_bus::Cyw43Bus::new(pio_spi),
            wl_dio: gpio::Pin(wl_dio),
            wl_cs: gpio::Pin(wl_cs),
            wl_on: gpio::Pin(wl_on),
            bus_is_up: false,
        }
    }

    fn init(&self) -> Result<(), device_driver::DevError> {
        self.ll_init()?;

        Ok(())
    }

    fn init_hw(
        &mut self,
        pio0: pac::PIO0,
        resets: &mut pac::RESETS,
    ) -> Result<(), device_driver::DevError> {
        // Init PIO SPI pins/program while CYW43 is still off.
        self.bus.init_hw(pio0, resets)?;

        Ok(())
    }

    fn gpio_config(&self) {
        // WL_ON
        self.gpio.pin_config(
            self.wl_on.0,
            gpio::Function::SIO,
            gpio::Pull::Up,
            Some(gpio::Direction::Output),
        );

        // WL_DIO
        self.gpio.pin_config(
            self.wl_dio.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Output),
        );
        self.gpio.set_level(&self.wl_dio, gpio::Level::Low);

        // WL_CS
        self.gpio.pin_config(
            self.wl_cs.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Output),
        );
        self.gpio.set_level(&self.wl_cs, gpio::Level::High);
    }

    fn spi_reset(&self) -> Result<(), device_driver::DevError> {
        // Set WL_ON low
        self.gpio.set_level(&self.wl_on, gpio::Level::Low);
        delay_ms(20);

        // Set WL_ON high
        self.gpio.set_level(&self.wl_on, gpio::Level::High);
        delay_ms(250);

        // Set IRQ (WL_DIO) high
        self.gpio.pin_config(
            self.wl_dio.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Input),
        );

        Ok(())
    }

    fn ll_init(&self) -> Result<(), device_driver::DevError> {
        Ok(())
    }

    fn ensure_up(&mut self) -> Result<(), device_driver::DevError> {
        if self.bus_is_up {
            return Ok(());
        }

        // Reset and power up the WL chip
        self.gpio.set_level(&self.wl_on, gpio::Level::Low);
        delay_ms(20);
        self.gpio.set_level(&self.wl_on, gpio::Level::High);
        delay_ms(50);

        self.gpio_config();

        self.spi_reset()?;

        self.bus.gpio_setup()?;

        self.bus.init()?;

        self.bus_is_up = true;

        Ok(())
    }
}

pub struct Cyw43 {
    inner: IrqSafeNullLock<Cyw43Inner>,
}

impl Cyw43 {
    pub const COMPATIBLE: &'static str = "CYW43439 Wlan";

    pub const fn new(
        gpio: &'static dyn gpio::interface::Gpio,
        wl_clk: usize,
        wl_dio: usize,
        wl_cs: usize,
        wl_on: usize,
    ) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Cyw43Inner::new(gpio, wl_clk, wl_dio, wl_cs, wl_on)),
        }
    }

    pub fn init_hw(
        &self,
        pio0: pac::PIO0,
        resets: &mut pac::RESETS,
    ) -> Result<(), device_driver::DevError> {
        self.inner.lock(|inner| inner.init_hw(pio0, resets))
    }
}

impl device_driver::interface::Driver for Cyw43 {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), device_driver::DevError> {
        self.inner.lock(|inner| inner.init())
    }

    fn register_irq_handler(
        &'static self,
        _irq_number: Self::IrqNumberType,
    ) -> Result<(), &'static str> {
        Ok(())
    }
}

impl device_driver::interface::Device for Cyw43 {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        self.inner.lock(|inner| {
            inner.ensure_up()?;
            Ok(4)
        })
    }
}

impl device_driver::interface::DeviceDriver for Cyw43 {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl interrupt::interface::IrqHandler for Cyw43 {
    fn handler(&self) -> Result<(), &'static str> {
        Ok(())
    }
}
