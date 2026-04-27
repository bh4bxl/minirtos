use crate::{
    bsp::pac,
    drivers::gpio::{Direction, Pull, interface::Gpio},
    sys::{
        device_driver,
        synchronization::{NullLock, interface::Mutex},
    },
};

use super::{Function, Level, Pin};

struct Rp235xGpioInner {
    io_bank0_regs: *const pac::io_bank0::RegisterBlock,
    pads_bank0_regs: *const pac::pads_bank0::RegisterBlock,
    sio_regs: *const pac::sio::RegisterBlock,
}

#[allow(dead_code)]
impl Rp235xGpioInner {
    const fn new() -> Self {
        Self {
            io_bank0_regs: unsafe { &*pac::IO_BANK0::ptr() },
            pads_bank0_regs: unsafe { &*pac::PADS_BANK0::ptr() },
            sio_regs: unsafe { &*pac::SIO::ptr() },
        }
    }

    #[inline]
    fn io_bank0_regs(&self) -> &pac::io_bank0::RegisterBlock {
        unsafe { &*self.io_bank0_regs }
    }

    #[inline]
    fn pads_bank0_regs(&self) -> &pac::pads_bank0::RegisterBlock {
        unsafe { &*self.pads_bank0_regs }
    }

    #[inline]
    fn sio_regs(&self) -> &pac::sio::RegisterBlock {
        unsafe { &*self.sio_regs }
    }

    fn eable(&self, pin: &Pin, enable: bool) {
        let pin = pin.0;
        assert!(pin < 30);

        let pad = self.pads_bank0_regs().gpio(pin);
        if enable {
            pad.modify(|_, w| w.iso().clear_bit());
        } else {
            pad.modify(|_, w| w.iso().set_bit());
        }
    }

    fn set_function(&self, pin: &Pin, func: Function) {
        let pin = pin.0;
        assert!(pin < 30);

        // Configure pad electrical state first
        self.pads_bank0_regs().gpio(pin).modify(|_, w| {
            // Enable input buffer for peripheral pins.
            // UART RX / SPI MISO need it; TX/SCK/MOSI usually harmless.
            w.ie().set_bit();

            // Make sure output is not disabled.
            // OD = output disable. clear = output allowed.
            w.od().clear_bit()
        });

        // Select peripheral function
        self.io_bank0_regs()
            .gpio(pin)
            .gpio_ctrl()
            .modify(|_, w| unsafe { w.funcsel().bits(func as u8) });
    }

    fn set_pull(&self, pin: &Pin, pull: Pull) {
        let pin = pin.0;
        assert!(pin < 30);

        let pad = self.pads_bank0_regs().gpio(pin);
        match pull {
            Pull::None => {
                pad.modify(|_, w| {
                    w.pue().clear_bit();
                    w.pde().clear_bit()
                });
            }
            Pull::Up => {
                pad.modify(|_, w| {
                    w.pue().set_bit();
                    w.pde().clear_bit()
                });
            }
            Pull::Down => {
                pad.modify(|_, w| {
                    w.pue().clear_bit();
                    w.pde().set_bit()
                });
            }
        }
    }

    fn set_direction(&self, pin: &Pin, direction: Direction, enable: bool) {
        let pin = pin.0;
        assert!(pin < 30);

        let pad = self.pads_bank0_regs().gpio(pin);
        match direction {
            Direction::Input => {
                pad.modify(|_, w| {
                    if enable {
                        w.ie().set_bit();
                        w.od().set_bit()
                    } else {
                        w.ie().clear_bit();
                        w.od().set_bit()
                    }
                });
                self.sio_regs()
                    .gpio_oe_clr()
                    .write(|w| unsafe { w.bits(1u32 << pin) });
            }
            Direction::Output => {
                pad.modify(|_, w| {
                    if enable {
                        w.ie().set_bit();
                        w.od().clear_bit()
                    } else {
                        w.od().set_bit()
                    }
                });
                if enable {
                    self.sio_regs()
                        .gpio_oe_set()
                        .write(|w| unsafe { w.bits(1u32 << pin) });
                } else {
                    self.sio_regs()
                        .gpio_oe_clr()
                        .write(|w| unsafe { w.bits(1u32 << pin) });
                }
            }
        }
    }

    fn set_level(&self, pin: &Pin, level: Level) {
        let pin = pin.0;

        match level {
            Level::High => {
                self.sio_regs()
                    .gpio_out_set()
                    .write(|w| unsafe { w.bits(1u32 << pin) });
            }
            Level::Low => {
                self.sio_regs()
                    .gpio_out_clr()
                    .write(|w| unsafe { w.bits(1u32 << pin) });
            }
        }
    }

    fn get_level(&self, pin: &Pin) -> Level {
        let pin = pin.0;
        if self.sio_regs().gpio_in().read().bits() & (1u32 << pin) == 0 {
            Level::Low
        } else {
            Level::High
        }
    }
}

pub struct Rp235xGpio {
    inner: NullLock<Rp235xGpioInner>,
}

impl Rp235xGpio {
    pub const COMPATIBLE: &'static str = "RP235x GPIO";

    pub const fn new() -> Self {
        Self {
            inner: NullLock::new(Rp235xGpioInner::new()),
        }
    }
}

impl Gpio for Rp235xGpio {
    fn eable(&self, pin: &Pin, enable: bool) {
        self.inner.lock(|inner| inner.eable(pin, enable));
    }

    fn set_function(&self, pin: &Pin, func: Function) {
        self.inner.lock(|inner| inner.set_function(pin, func));
    }

    fn set_pull(&self, pin: &Pin, pull: Pull) {
        self.inner.lock(|inner| inner.set_pull(pin, pull));
    }

    fn set_direction(&self, pin: &Pin, direction: Direction, enable: bool) {
        self.inner
            .lock(|inner| inner.set_direction(pin, direction, enable));
    }

    fn set_level(&self, pin: &Pin, level: Level) {
        self.inner.lock(|inner| inner.set_level(pin, level));
    }

    fn get_level(&self, pin: &Pin) -> Level {
        self.inner.lock(|inner| inner.get_level(pin))
    }

    fn pin_config(&self, pin: usize, func: Function, pull: Pull, direction: Option<Direction>) {
        self.inner.lock(|inner| {
            let pin = Pin(pin as usize);
            inner.eable(&pin, true);
            inner.set_function(&pin, func);
            inner.set_pull(&pin, pull);
            if let Some(direction) = direction {
                inner.set_direction(&pin, direction, true);
            }
        });
    }
}

impl device_driver::interface::Driver for Rp235xGpio {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }
}

impl device_driver::interface::Device for Rp235xGpio {
    fn read(&self, data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        if data.len() < 2 {
            return Err(device_driver::DevError::InvalidArg);
        }

        let pin = Pin(data[0] as usize);

        data[1] = match self.get_level(&pin) {
            Level::Low => 0,
            Level::High => 1,
        };

        Ok(2)
    }

    fn write(&self, data: &[u8]) -> Result<usize, device_driver::DevError> {
        if data.len() < 2 {
            return Err(device_driver::DevError::InvalidArg);
        }

        let pin = Pin(data[0] as usize);

        match data[1] {
            0 => self.set_level(&pin, Level::Low),
            _ => self.set_level(&pin, Level::High),
        }

        Ok(2)
    }
}

impl device_driver::interface::DeviceDriver for Rp235xGpio {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}
