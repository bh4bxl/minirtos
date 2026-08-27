use minirtos_abi::{MessageData, ServiceId, SysError};
use minirtos_kernel::sys::{Endpoint, Read, Service, Write};

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
            endpoint: Service::lookup(id.service_id())?,
        })
    }

    pub fn write_byte(&mut self, byte: u8) -> Result<(), SysError> {
        let message = MessageData::new(UartOp::WriteByte as u32, [byte as u32, 0, 0, 0]);

        self.endpoint.send(&message)
    }

    pub fn config(&self, config: &UartConfig) -> Result<(), SysError> {
        let buf = unsafe {
            core::slice::from_raw_parts(
                config as *const UartConfig as *const u8,
                core::mem::size_of::<UartConfig>(),
            )
        };

        self.endpoint.write(UartOp::Config as u32, buf)?;

        Ok(())
    }
}

impl Write for Uart {
    type Error = SysError;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.endpoint.write(UartOp::Write as u32, buf)
    }
}

impl Read for Uart {
    type Error = SysError;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.endpoint.read(UartOp::Read as u32, buf)
    }
}
