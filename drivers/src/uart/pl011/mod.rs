use core::fmt::Write;

use minirtos_kernel::{
    MemoryBlock,
    interface::{
        IoRead, IoWrite,
        driver::{Driver, DriverConfig},
    },
    synchronization::{IrqLock, Lock},
};
use minirtos_services::driver::{UartDriver, uart::UartConfig};

use crate::DevError;

mod inner;
mod registers;

use inner::Pl011Inner;

#[derive(PartialEq)]
enum BlockingMode {
    Blocking,
    NonBlocking,
}

pub struct Pl011<I, D>
where
    I: Copy,
    D: Copy,
{
    inner: IrqLock<Pl011Inner<I, D>>,
}

impl<I, D> Driver for Pl011<I, D>
where
    I: Copy,
    D: Copy,
{
    type Interrupt = I;
    type Dma = D;
    type Error = DevError;

    fn new(config: DriverConfig<Self::Interrupt, Self::Dma>) -> Result<Self, Self::Error> {
        let inner = Pl011Inner::new(config)?;

        Ok(Self {
            inner: IrqLock::new(inner),
        })
    }

    fn device_memory_blocks(&self) -> &[MemoryBlock] {
        &self.inner.lock(|inner| inner.device_memory_blocks())
    }
}

impl<I, D> UartDriver for Pl011<I, D>
where
    I: Copy,
    D: Copy,
{
    type Error = DevError;

    fn init(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn config(&self, config: &UartConfig) -> Result<(), Self::Error> {
        self.inner.lock(|inner| {
            inner.disable();

            inner.clear_all_interrupts();

            inner.set_baudrate(config.clock_hz, config.baud_rate)?;

            inner.configure_line_control(config.data_bits, config.stop_bits, config.parity);

            inner.enable();

            Ok(())
        })
    }

    fn read_byte(&self) -> Result<u8, Self::Error> {
        self.inner
            .lock(|inner| inner.read_byte(BlockingMode::Blocking))
            .ok_or(DevError::Io)
    }

    fn try_read_byte(&self) -> Result<Option<u8>, Self::Error> {
        Ok(self
            .inner
            .lock(|inner| inner.read_byte(BlockingMode::NonBlocking)))
    }

    fn write_byte(&self, byte: u8) -> Result<(), Self::Error> {
        self.inner.lock(|inner| inner.write_byte(byte));

        Ok(())
    }

    fn write_buf(&self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.lock(|inner| {
            for &c in buf {
                inner.write_byte(c);
            }
        });

        Ok(buf.len())
    }

    fn flush(&self) -> Result<(), Self::Error> {
        self.inner.lock(|inner| inner.flush());

        Ok(())
    }
}

impl<I, D> IoWrite for Pl011<I, D>
where
    I: Copy,
    D: Copy,
{
    fn write_char(&self, c: char) {
        self.inner.lock(|inner| inner.write_byte(c as u8))
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        self.inner.lock(|inner| inner.write_fmt(args))
    }

    fn flush(&self) {
        self.inner.lock(|inner| inner.flush())
    }
}

impl<I, D> IoRead for Pl011<I, D>
where
    I: Copy,
    D: Copy,
{
    fn try_read_char(&self) -> Option<char> {
        self.inner
            .lock(|inner| inner.read_byte(BlockingMode::NonBlocking).map(char::from))
    }

    fn read_char(&self) -> char {
        self.inner
            .lock(|inner| inner.read_byte(BlockingMode::Blocking).unwrap() as char)
    }

    fn clear_input(&self) {
        while self
            .inner
            .lock(|inner| inner.read_byte(BlockingMode::NonBlocking))
            .is_some()
        {}
    }
}
