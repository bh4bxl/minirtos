use minirtos_abi::{EndpointHandle, ReceivedRequest, ServiceId, ServiceOp, SysError, SyscallId};

use crate::{arch::syscall, task::TaskId};

use super::super::Endpoint;

pub struct Service {
    id: ServiceId,
    endpoint: Endpoint,
}

impl Service {
    pub fn new(id: ServiceId) -> Result<Self, SysError> {
        Ok(Self {
            id,
            endpoint: Endpoint::create()?,
        })
    }

    pub fn register(&self) -> Result<(), SysError> {
        let ret = syscall::<{ SyscallId::Service as u8 }>(&[
            ServiceOp::Register as u32,
            self.id.raw(),
            self.endpoint.handle().raw(),
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }

    pub fn lookup(id: ServiceId) -> Result<Endpoint, SysError> {
        let ret =
            syscall::<{ SyscallId::Service as u8 }>(&[ServiceOp::Lookup as u32, id.raw()]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(Endpoint::from_handle(EndpointHandle::from_raw(ret as u32)))
        }
    }

    pub fn recv(&self) -> Result<ReceivedRequest, SysError> {
        self.endpoint.recv()
    }

    pub fn complete(
        &self,
        sender: TaskId,
        result: Result<usize, SysError>,
    ) -> Result<(), SysError> {
        self.endpoint.complete(sender, result)
    }

    pub fn unregister(&self) -> Result<(), SysError> {
        let ret =
            syscall::<{ SyscallId::Service as u8 }>(&[ServiceOp::Unregister as u32, self.id.raw()])
                as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }
}
