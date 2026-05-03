use rp235x_pac as pac;

use crate::{
    drivers::{delay_ms, gpio},
    sys::{
        device_driver, interrupt,
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

pub mod firmware;
pub mod pio_ctrl;
pub mod pio_spi;

struct Cyw43Inner {
    gpio: &'static dyn gpio::interface::Gpio,
    pio_spi: pio_spi::PioSpi,

    wl_on: gpio::Pin,
}

impl Cyw43Inner {
    const fn new(
        gpio: &'static dyn gpio::interface::Gpio,
        wl_clk: usize,
        wl_d: usize,
        wl_cs: usize,
        wl_on: usize,
    ) -> Self {
        Self {
            gpio,
            pio_spi: pio_spi::PioSpi::new(gpio, wl_clk, wl_d, wl_cs),
            wl_on: gpio::Pin(wl_on),
        }
    }

    fn init(&self) -> Result<(), device_driver::DevError> {
        // Wlan ON
        self.gpio.pin_config(
            self.wl_on.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Output),
        );
        self.wl_on_low();
        delay_ms(20);

        // Power on CYW43.
        self.wl_on_high();
        delay_ms(250);

        // PIO SPI Pins
        self.pio_spi.init();

        Ok(())
    }

    fn init_hw(
        &mut self,
        pio0: pac::PIO0,
        resets: &mut pac::RESETS,
    ) -> Result<(), device_driver::DevError> {
        // Init PIO SPI pins/program while CYW43 is still off.
        self.pio_spi.init_hw(pio0, resets);

        Ok(())
    }

    #[inline]
    fn wl_on_low(&self) {
        self.gpio.set_level(&self.wl_on, gpio::Level::Low);
    }

    #[inline]
    fn wl_on_high(&self) {
        self.gpio.set_level(&self.wl_on, gpio::Level::High);
    }

    #[inline]
    fn make_cmd(&self, write: bool, inc: bool, func: u32, addr: u32, size: u32) -> u32 {
        ((write as u32) << 31)
            | ((inc as u32) << 30)
            | ((func & 0x3) << 28)
            | ((addr & 0x1ffff) << 11)
            | (size & 0x7ff)
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
        wl_d: usize,
        wl_cs: usize,
        wl_on: usize,
    ) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Cyw43Inner::new(gpio, wl_clk, wl_d, wl_cs, wl_on)),
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
            let cmd = inner.make_cmd(false, true, 1, 0x0014, 4);
            let tx = cmd.to_be_bytes();
            for d in tx.iter() {
                defmt::info!("write {:x}", d);
            }
            let mut rx = [0u8; 8];
            rx[0..4].copy_from_slice(&cmd.to_be_bytes());
            inner.pio_spi.transfer(&tx, &mut rx)?;
            // let mut rx = [];
            // inner.pio_spi.transfer(&tx, &mut rx)?;
            for d in rx.iter() {
                defmt::info!("read {:x}", d);
            }
            Ok(rx.len())
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
