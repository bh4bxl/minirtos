use core::fmt::{self, Write};

use cortex_m::asm;

use crate::{
    drivers::uart::{Config, Parity, interface},
    sys::{
        console,
        device_driver::{self, DeviceIrq, DeviceIrqCallback, DeviceIrqEvent},
        interrupt::{self, IrqHandlerDescriptor, irq_manager},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

use crate::bsp::pac;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UartId {
    UART0,
    UART1,
}

#[allow(dead_code)]
pub struct Pl011UartInner {
    id: UartId,
    regs: *const pac::uart0::RegisterBlock,
    irq_callback: Option<DeviceIrqCallback>,
}

#[allow(dead_code)]
impl Pl011UartInner {
    /// Create an instance
    const fn new(id: UartId) -> Self {
        let regs = match id {
            UartId::UART0 => unsafe { &*pac::UART0::ptr() },
            UartId::UART1 => unsafe { &*pac::UART1::ptr() },
        };

        Self {
            id,
            regs,
            irq_callback: None,
        }
    }

    fn id(&self) -> UartId {
        self.id
    }

    fn regs(&self) -> &pac::uart0::RegisterBlock {
        unsafe { &*self.regs }
    }

    fn init(&self) {
        let resets = unsafe { &*pac::RESETS::ptr() };

        // clear uart reset
        resets.reset().modify(|_, w| match self.id {
            UartId::UART0 => w.uart0().clear_bit(),
            UartId::UART1 => w.uart1().clear_bit(),
        });

        // Wait for reset done
        match self.id {
            UartId::UART0 => {
                while resets.reset_done().read().bits() & (1 << 26) == 0 {
                    asm::nop();
                }
            }
            UartId::UART1 => {
                while resets.reset_done().read().bits() & (1 << 27) == 0 {
                    asm::nop();
                }
            }
        }
    }

    fn config(&self, config: &Config) {
        self.enable(false);

        self.clear_all_interrupts();

        self.set_baudrate(config.clock_hz, config.baudrate);

        self.configure_line_control(config);

        self.eable_irq(config.eable_irq);

        self.enable(true);
    }

    fn write_byte(&self, byte: u8) {
        while self.tx_fifo_full() {}

        self.regs()
            .uartdr()
            .write(|w| unsafe { w.bits(byte as u32) });
    }

    fn read_byte(&self, blocking: bool) -> Option<u8> {
        if self.regs().uartfr().read().rxfe().bit_is_set() {
            if !blocking {
                return None;
            }

            while self.regs().uartfr().read().rxfe().bit_is_set() {
                asm::nop();
            }
        }

        let ret = self.regs().uartdr().read().bits() as u8;
        Some(ret)
    }

    fn enable(&self, enable: bool) {
        if enable {
            self.regs().uartcr().modify(|_, w| {
                w.uarten().set_bit();
                w.txe().set_bit();
                w.rxe().set_bit()
            });
        } else {
            self.regs().uartcr().modify(|_, w| {
                w.uarten().clear_bit();
                w.txe().clear_bit();
                w.rxe().clear_bit()
            });
        }
    }

    fn clear_all_interrupts(&self) {
        self.regs().uarticr().write(|w| unsafe { w.bits(0x07ff) });
    }

    fn set_baudrate(&self, uart_clock_hz: u32, baudrate: u32) {
        let baud_x64 = ((4 * uart_clock_hz) + (baudrate / 2)) / baudrate;
        let ibrd = baud_x64 / 64;
        let fbrd = baud_x64 % 64;

        self.regs().uartibrd().write(|w| unsafe { w.bits(ibrd) });
        self.regs().uartfbrd().write(|w| unsafe { w.bits(fbrd) });
    }

    fn configure_line_control(&self, config: &Config) {
        self.regs().uartlcr_h().modify(|_, w| {
            // FIFO enable
            w.fen().set_bit();
            // Word length
            if config.data_bits < 5 || config.data_bits > 8 {
                panic!("Unsupported data_bits");
            }
            unsafe {
                w.wlen().bits(config.data_bits - 5);
            }
            // Stop bits
            if config.stop_bits == 2 {
                w.stp2().set_bit();
            } else {
                w.stp2().clear_bit();
            }
            // Parity
            match config.parity {
                Parity::None => {
                    w.pen().clear_bit();
                    w.eps().clear_bit()
                }
                Parity::Even => {
                    w.pen().set_bit();
                    w.eps().set_bit()
                }
                Parity::Odd => {
                    w.pen().set_bit();
                    w.eps().clear_bit()
                }
            }
        });
    }

    fn eable_irq(&self, enable: bool) {
        if enable {
            self.regs().uartimsc().modify(|_, w| {
                w.rxim().set_bit();
                w.rtim().set_bit()
            });
        } else {
            self.regs().uartimsc().modify(|_, w| {
                w.rxim().clear_bit();
                w.rtim().clear_bit()
            });
        }
    }

    /// Block execution until the last buffered character has been physically put on the TX wire.
    fn flush(&self) {
        while self.regs().uartfr().read().busy().bit_is_set() {
            asm::nop();
        }
    }

    fn tx_fifo_full(&self) -> bool {
        self.regs().uartfr().read().txff().bit_is_set()
    }

    fn rx_fifo_empty(&self) -> bool {
        self.regs().uartfr().read().rxfe().bit_is_set()
    }
}

/// Implementing `core::fmt::Write`
impl fmt::Write for Pl011UartInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_byte(c as u8);
        }
        Ok(())
    }
}

pub struct Pl011Uart {
    inner: IrqSafeNullLock<Pl011UartInner>,
}

impl Pl011Uart {
    pub const COMPATIBLE: &'static str = "RP235x PL011 UART";

    /// Create an instance
    pub const fn new(id: UartId) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Pl011UartInner::new(id)),
        }
    }
}

impl interface::Uart for Pl011Uart {
    fn config(&self, config: &Config) {
        self.inner.lock(|inner| inner.config(config));
    }
}

/// Device driver for PL011 UART
impl device_driver::interface::Driver for Pl011Uart {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| inner.init());
        Ok(())
    }

    fn register_irq_handler(
        &'static self,
        irq_number: Self::IrqNumberType,
    ) -> Result<(), &'static str> {
        let descriptor = IrqHandlerDescriptor::new(irq_number, Self::COMPATIBLE, self);

        irq_manager().register_irq_handler(descriptor)
    }
}

impl device_driver::interface::Device for Pl011Uart {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, data: &[u8]) -> Result<usize, device_driver::DevError> {
        self.inner.lock(|inner| {
            for &b in data {
                inner.write_byte(b);
            }
        });
        Ok(data.len())
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

impl device_driver::interface::DeviceDriver for Pl011Uart {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

/// Console for PL011 UART
impl console::interface::Write for Pl011Uart {
    fn write_char(&self, c: char) {
        self.inner.lock(|inner| inner.write_byte(c as u8));
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        self.inner.lock(|inner| inner.write_fmt(args))
    }

    fn flush(&self) {
        self.inner.lock(|inner| inner.flush());
    }
}

impl console::interface::Read for Pl011Uart {
    fn read_char(&self) -> char {
        self.inner
            .lock(|inner| inner.read_byte(true).unwrap() as char)
    }

    fn clear_rx(&self) {
        while self.inner.lock(|inner| inner.read_byte(false)).is_some() {}
    }
}

impl console::interface::All for Pl011Uart {}

impl interrupt::interface::IrqHandler for Pl011Uart {
    fn handler(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| {
            let mis = inner.regs().uartmis().read();

            if mis.rxmis().bit_is_set() || mis.rtmis().bit_is_set() {
                while !inner.regs().uartfr().read().rxfe().bit_is_set() {
                    let b = inner.regs().uartdr().read().bits() as u8;

                    if let Some(cb) = inner.irq_callback {
                        cb(DeviceIrq {
                            event: DeviceIrqEvent::RxReady,
                            data: b as usize,
                        });
                    }
                }
            }

            // Clear handled RX / RX timeout interrupts
            inner.regs().uarticr().write(|w| {
                w.rxic().bit(true);
                w.rtic().bit(true)
            });
        });
        Ok(())
    }
}
