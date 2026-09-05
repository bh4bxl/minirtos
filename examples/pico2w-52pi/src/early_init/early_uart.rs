use core::fmt::{self, Write};

use alloc::vec;
use minirtos_drivers::{DevError, uart::UartPl011};
use minirtos_kernel::MemoryBlock;
use minirtos_services::driver::{DriverConfig, UartDriver, interface::Driver, uart::UartConfig};

pub(super) fn init(sys_clk: u32) -> Result<(), DevError> {
    let dev_cfg = DriverConfig::<(), ()> {
        dev_mem_blocks: vec![MemoryBlock::new(0x4007_0000, 0x1000)],
        interrupts: vec![],
        dmas: vec![],
    };

    let mut config = UartConfig::default();
    config.clock_hz = sys_clk;

    let mut uart = UartPl011::new(dev_cfg)?;

    uart.config(&config)?;

    Ok(())
}

pub(super) fn write(buf: &[u8]) {
    let dev_cfg = DriverConfig::<(), ()> {
        dev_mem_blocks: vec![MemoryBlock::new(0x4007_0000, 0x1000)],
        interrupts: vec![],
        dmas: vec![],
    };

    if let Ok(uart) = UartPl011::new(dev_cfg) {
        let _ = uart.write_buf(buf);
    }
}

struct EarlyUart;

impl Write for EarlyUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(s.as_bytes());
        Ok(())
    }
}

pub fn write_fmt(args: fmt::Arguments<'_>) {
    let _ = EarlyUart.write_fmt(args);
}

#[macro_export]
macro_rules! early_print {
    ($($arg:tt)*) => {
        $crate::early_uart::write_fmt(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! early_println {
    () => {
        $crate::early_print!("\r\n")
    };

    ($($arg:tt)*) => {
        $crate::early_init::early_uart::write_fmt(
            core::format_args!("{}\r\n", core::format_args!($($arg)*))
        )
    };
}
