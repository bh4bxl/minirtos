use core::fmt::Write;

use heapless::String;

use crate::sys::{
    device_driver::{self, DeviceIrqEvent},
    sync::message_queue::MessageQueue,
};

pub static CONSOLE_TX_Q: MessageQueue<u8, 512> = MessageQueue::new();

pub static CONSOLE_RX_Q: MessageQueue<u8, 512> = MessageQueue::new();

/// Uart IRQ Callback
fn console_uart_irq_callback(irq: crate::sys::device_driver::DeviceIrq) {
    if irq.event == DeviceIrqEvent::RxReady {
        let b = irq.data as u8;
        let _ = CONSOLE_RX_Q.try_send(b);
    }
}

/// The console thread
pub extern "C" fn queue_console_task(_arg: *mut ()) -> ! {
    let uart = match device_driver::driver_manager().open_device(device_driver::DeviceType::Uart, 0)
    {
        Some(dev) => dev,
        None => loop {
            defmt::warn!("No uart device found");
            cortex_m::asm::wfi();
        },
    };

    uart.set_irq_callback(Some(console_uart_irq_callback)).ok();

    loop {
        let b = CONSOLE_TX_Q.recv();

        let _ = uart.write(&[b]);
    }
}

pub struct QueueConsole;

impl QueueConsole {
    pub const fn new() -> Self {
        Self {}
    }
}

impl super::interface::Write for QueueConsole {
    fn write_char(&self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        for &b in s.as_bytes() {
            CONSOLE_TX_Q.send(b);
        }
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        let mut s = String::<128>::new();
        s.write_fmt(args)?;

        for &b in s.as_bytes() {
            CONSOLE_TX_Q.send(b);
        }

        Ok(())
    }

    fn flush(&self) {}
}

impl super::interface::Read for QueueConsole {
    fn read_char(&self) -> char {
        let b = CONSOLE_RX_Q.recv();
        b as char
    }

    fn clear_rx(&self) {}
}

impl super::interface::All for QueueConsole {}
