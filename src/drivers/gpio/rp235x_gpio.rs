use crate::{
    bsp::pac,
    drivers::gpio::{Direction, GpioIrqHandler, Pull, interface::Gpio},
    sys::{
        device_driver::{self, DeviceIrq, DeviceIrqCallback, DeviceIrqEvent},
        interrupt::{self, IrqHandlerDescriptor},
        synchronization::{NullLock, interface::Mutex},
    },
};

use super::{Function, Level, Pin};

const MAX_PINS: usize = 30;

struct Rp235xGpioInner {
    io_bank0_regs: *const pac::io_bank0::RegisterBlock,
    pads_bank0_regs: *const pac::pads_bank0::RegisterBlock,
    sio_regs: *const pac::sio::RegisterBlock,
    irq_handlers: [Option<GpioIrqHandler>; MAX_PINS],
    irq_callback: Option<DeviceIrqCallback>,
}

#[allow(dead_code)]
impl Rp235xGpioInner {
    const fn new() -> Self {
        Self {
            io_bank0_regs: unsafe { &*pac::IO_BANK0::ptr() },
            pads_bank0_regs: unsafe { &*pac::PADS_BANK0::ptr() },
            sio_regs: unsafe { &*pac::SIO::ptr() },
            irq_handlers: [None; MAX_PINS],
            irq_callback: None,
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
        assert!(pin < MAX_PINS);

        let pad = self.pads_bank0_regs().gpio(pin);
        if enable {
            pad.modify(|_, w| w.iso().clear_bit());
        } else {
            pad.modify(|_, w| w.iso().set_bit());
        }
    }

    fn set_function(&self, pin: &Pin, func: Function) {
        let pin = pin.0;
        assert!(pin < MAX_PINS);

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
        assert!(pin < MAX_PINS);

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
        assert!(pin < MAX_PINS);

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
        assert!(pin < MAX_PINS);

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

    fn set_input_hysteresis(&self, pin: &Pin, enable: bool) {
        self.pads_bank0_regs()
            .gpio(pin.0)
            .modify(|_, w| w.schmitt().bit(enable));
    }

    fn enable_irq(&self, pin: &Pin, trigger: super::GpioIrqTrigger, _debounce_ms: u32) {
        let pin = pin.0;
        assert!(pin < MAX_PINS);

        // clear old pending first
        let bank = pin / 8;
        let shift = (pin % 8) * 4;
        self.io_bank0_regs()
            .intr(bank)
            .write(|w| unsafe { w.bits(0x0f << shift) });

        // enable selected interrupt for proc0
        let mask = match trigger {
            super::GpioIrqTrigger::LevelLow => 1u32 << (shift + 0),
            super::GpioIrqTrigger::LevelHigh => 1u32 << (shift + 1),
            super::GpioIrqTrigger::EdgeLow => 1u32 << (shift + 2),
            super::GpioIrqTrigger::EdgeHigh => 1u32 << (shift + 3),
            super::GpioIrqTrigger::EdgeBoth => (1u32 << (shift + 2)) | (1u32 << (shift + 3)),
        };
        self.io_bank0_regs()
            .proc0_inte(bank)
            .modify(|r, w| unsafe { w.bits(r.bits() | mask) });
    }

    fn register_irq_handler(&mut self, pin: &Pin, handler: Option<GpioIrqHandler>) {
        let pin = pin.0;
        assert!(pin < MAX_PINS);

        self.irq_handlers[pin] = handler;
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

    fn set_input_hysteresis(&self, pin: &Pin, enable: bool) {
        self.inner
            .lock(|inner| inner.set_input_hysteresis(pin, enable));
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

    fn enable_irq(&self, pin: &Pin, trigger: super::GpioIrqTrigger, debounce_ms: u32) {
        self.inner
            .lock(|inner| inner.enable_irq(pin, trigger, debounce_ms));
    }

    fn register_irq_handler(&self, pin: &Pin, handler: Option<GpioIrqHandler>) {
        self.inner
            .lock(|inner| inner.register_irq_handler(pin, handler));
    }
}

impl device_driver::interface::Driver for Rp235xGpio {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn register_irq_handler(
        &'static self,
        irq_number: Self::IrqNumberType,
    ) -> Result<(), &'static str> {
        let descriptor = IrqHandlerDescriptor::new(irq_number, Self::COMPATIBLE, self);

        interrupt::irq_manager().register_irq_handler(descriptor)
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

    fn set_irq_callback(
        &self,
        cb: Option<DeviceIrqCallback>,
    ) -> Result<(), device_driver::DevError> {
        self.inner.lock(|inner| {
            inner.irq_callback = cb;
        });

        Ok(())
    }
}

impl device_driver::interface::DeviceDriver for Rp235xGpio {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl interrupt::interface::IrqHandler for Rp235xGpio {
    fn handler(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| {
            for bank in 0..4 {
                let status = inner.io_bank0_regs().proc0_ints(bank).read().bits();
                if status == 0 {
                    continue;
                }

                for i in 0..8 {
                    let shift = i * 4;
                    let bits = (status >> shift) & 0x0f;

                    if bits != 0 {
                        let pin = bank * 8 + i;

                        // Clear interrupt
                        inner
                            .io_bank0_regs()
                            .intr(bank)
                            .write(|w| unsafe { w.bits(bits << shift) });

                        // Get GPIO level
                        let level = inner.get_level(&Pin(pin));

                        // To driver layer
                        if let Some(handler) = inner.irq_handlers[pin] {
                            handler(&Pin(pin), level);
                        }

                        // To upper layer
                        let level = match level {
                            Level::Low => 0,
                            Level::High => 1,
                        };

                        if let Some(cb) = inner.irq_callback {
                            cb(DeviceIrq {
                                event: DeviceIrqEvent::Gpio,
                                data: pin | level << 8,
                            });
                        }
                    }
                }
            }
        });
        Ok(())
    }
}
