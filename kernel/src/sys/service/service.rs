use minirtos_abi::{EndpointHandle, ServiceId, ServiceOp, SysError, SyscallId};

use crate::arch::syscall;

use super::super::Endpoint;

pub struct Service;

impl Service {
    pub fn register(id: ServiceId, endpoint: &Endpoint) -> Result<(), SysError> {
        let ret = syscall::<{ SyscallId::Service as u8 }>(&[
            ServiceOp::Register as u32,
            id.raw(),
            endpoint.handle().raw(),
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

    pub fn unregister(id: ServiceId) -> Result<(), SysError> {
        let ret = syscall::<{ SyscallId::Service as u8 }>(&[ServiceOp::Unregister as u32, id.raw()])
            as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }
}
