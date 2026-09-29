use minirtos_abi::{MessageData, ServiceId, SysError};
use minirtos_kernel::{
    interface::{IoRead, IoWrite},
    sys::{Endpoint, ServiceEndpoint, SharedBuffer},
};

use super::{UartConfig, UartOp};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UartId(ServiceId);

impl UartId {
    pub const fn new(id: u32) -> Self {
        Self(ServiceId::from_raw(id))
    }

    pub(crate) const fn service_id(self) -> ServiceId {
        self.0
    }
}

pub struct Uart {
    endpoint: Endpoint,
}

impl Uart {
    pub fn open(id: UartId) -> Result<Self, SysError> {
        Ok(Self {
            endpoint: ServiceEndpoint::lookup(id.service_id())?,
        })
    }

    fn write(&self, data: &[u8]) -> Result<(), SysError> {
        if data.is_empty() {
            return Ok(());
        }

        let mut shared = SharedBuffer::alloc(data.len())?;

        shared.as_mut_slice().copy_from_slice(data);

        let message = MessageData::new(
            UartOp::Write as u32,
            [shared.handle().raw(), 0, data.len() as u32, 0],
        );

        self.endpoint.call(&message)?;

        shared.free()?;

        Ok(())
    }

    fn read_byte(&self) -> Result<u8, SysError> {
        let message = MessageData::new(UartOp::ReadByte as u32, [0; 4]);

        let response = self.endpoint.call(&message)?;

        let value = response.args[0] as i32;

        if value < 0 {
            Err(SysError::try_from(value).unwrap_or(SysError::InvalidState))
        } else {
            Ok(value as u8)
        }
    }

    fn try_read_byte(&self) -> Result<Option<u8>, SysError> {
        let message = MessageData::new(UartOp::TryReadByte as u32, [0; 4]);

        let response = self.endpoint.call(&message)?;

        let value = response.args[0] as i32;

        if value == SysError::WouldBlock as i32 {
            Ok(None)
        } else if value < 0 {
            Err(SysError::try_from(value).unwrap_or(SysError::InvalidState))
        } else {
            Ok(Some(value as u8))
        }
    }

    pub fn config(&self, config: &UartConfig) -> Result<(), SysError> {
        let size = core::mem::size_of::<UartConfig>();

        let mut shared = SharedBuffer::alloc(size)?;

        let src =
            unsafe { core::slice::from_raw_parts(config as *const UartConfig as *const u8, size) };

        shared.as_mut_slice().copy_from_slice(src);

        let message = MessageData::new(
            UartOp::Config as u32,
            [shared.handle().raw(), 0, size as u32, 0],
        );

        self.endpoint.call(&message)?;

        shared.free()?;

        Ok(())
    }
}

struct UartWriter<'a>(&'a Uart);

impl core::fmt::Write for UartWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

impl IoWrite for Uart {
    fn write_char(&self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        let _ = self.write(s.as_bytes());
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        use core::fmt::Write;

        UartWriter(self).write_fmt(args)
    }

    fn flush(&self) {
        let message = MessageData::new(UartOp::Flush as u32, [0; 4]);

        let _ = self.endpoint.call(&message);
    }
}

impl IoRead for Uart {
    fn read_char(&self) -> char {
        let first = match self.read_byte() {
            Ok(byte) => byte,
            Err(_) => return '\0',
        };

        if first < 0x80 {
            return first as char;
        }

        let len = if first & 0xe0 == 0xc0 {
            2
        } else if first & 0xf0 == 0xe0 {
            3
        } else if first & 0xf8 == 0xf0 {
            4
        } else {
            return '\u{fffd}';
        };

        let mut buf = [0u8; 4];
        buf[0] = first;

        for byte in &mut buf[1..len] {
            *byte = match self.read_byte() {
                Ok(byte) => byte,
                Err(_) => return '\u{fffd}',
            };
        }

        match core::str::from_utf8(&buf[..len]) {
            Ok(s) => s.chars().next().unwrap_or('\u{fffd}'),
            Err(_) => '\u{fffd}',
        }
    }

    fn try_read_char(&self) -> Option<char> {
        match self.try_read_byte() {
            Ok(Some(byte)) if byte.is_ascii() => Some(byte as char),
            _ => None,
        }
    }

    fn clear_input(&self) {}
}
