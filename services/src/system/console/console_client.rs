use minirtos_abi::{MessageData, ServiceId, SysError};
use minirtos_kernel::{
    interface::{IoRead, IoWrite},
    sys::{Endpoint, ServiceEndpoint, SharedBuffer},
};

use crate::system::console::ConsoleOp;

pub struct ConsoleClient {
    endpoint: Endpoint,
    tx: SharedBuffer,
    rx: SharedBuffer,
}

const CONSOLE_BUFFER_SIZE: usize = 256;

impl ConsoleClient {
    pub fn open(service_id: ServiceId) -> Result<Self, SysError> {
        let endpoint = ServiceEndpoint::lookup(service_id)?;

        let tx = SharedBuffer::alloc(CONSOLE_BUFFER_SIZE)?;
        let rx = SharedBuffer::alloc(CONSOLE_BUFFER_SIZE)?;

        let message = MessageData::new(
            ConsoleOp::Register as u32,
            [tx.handle().raw(), rx.handle().raw(), 0, 0],
        );

        endpoint.call(&message)?;

        Ok(Self { endpoint, tx, rx })
    }

    fn write(&self, data: &[u8]) -> Result<(), SysError> {
        if data.is_empty() {
            return Ok(());
        }

        if data.len() > self.tx.len() {
            return Err(SysError::InvalidArgument);
        }

        unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), self.tx.as_ptr() as *mut u8, data.len());
        }

        let message =
            MessageData::new(ConsoleOp::WritePending as u32, [data.len() as u32, 0, 0, 0]);

        self.endpoint.call(&message)?;

        Ok(())
    }
}

struct ConsoleWriter<'a>(&'a ConsoleClient);

impl core::fmt::Write for ConsoleWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

impl IoWrite for ConsoleClient {
    fn write_char(&self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        let _ = self.write(s.as_bytes());
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        use core::fmt::Write;

        ConsoleWriter(self).write_fmt(args)
    }

    fn flush(&self) {
        let message = MessageData::new(ConsoleOp::Flush as u32, [0; 4]);

        let _ = self.endpoint.call(&message);
    }
}

impl IoRead for ConsoleClient {
    fn read_char(&self) -> char {
        '\0'
    }

    fn try_read_char(&self) -> Option<char> {
        None
    }

    fn clear_input(&self) {}
}
