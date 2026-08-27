use minirtos_kernel::kinfo;
use minirtos_services::driver::{UartConfig, UartDriver};

use crate::DevError;

pub struct Pl011Config {
    pub base_addr: u32,
}

pub struct UartPl011 {
    base_addr: u32,
}

impl UartPl011 {
    pub const fn new() -> Self {
        Self {
            base_addr: 0x0000_0000,
        }
    }
}

impl UartDriver for UartPl011 {
    type Error = DevError;
    type Config = Pl011Config;

    fn init(&mut self, config: &Self::Config) -> Result<(), Self::Error> {
        kinfo!("pl011 init @ {:#010x}", config.base_addr);
        Ok(())
    }

    fn config(&mut self, config: &UartConfig) -> Result<(), Self::Error> {
        kinfo!("pl011 config baud_rate {}", config.baud_rate);
        kinfo!("pl011 config data_bits {}", config.data_bits as u8);
        kinfo!("pl011 config stop_bits {}", config.stop_bits as u8);
        kinfo!("pl011 config parity {}", config.parity as u8);
        Ok(())
    }

    fn try_read_byte(&self) -> Result<Option<u8>, Self::Error> {
        kinfo!("pl011 try_read_byte");
        Ok(Some(60u8))
    }

    fn write_byte(&self, byte: u8) -> Result<(), Self::Error> {
        kinfo!("pl011 write_byte {}", byte as char);
        Ok(())
    }

    fn write_buf(&self, buf: &[u8]) -> Result<usize, Self::Error> {
        if let Ok(s) = core::str::from_utf8(buf) {
            kinfo!("pl011 write_buf: {}", s);
        }
        Ok(buf.len())
    }
}
