use minirtos_abi::{MessageData, SharedBufferHandle, SysError};

use crate::service::{KernelServiceClass, MemoryOp, make_op};

use super::kernel_service_call;

pub struct SharedBuffer {
    handle: SharedBufferHandle,
    base: usize,
    size: usize,
}

impl SharedBuffer {
    pub fn alloc(size: usize) -> Result<Self, SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Memory as u16,
                MemoryOp::SharedBufferAlloc as u16,
            ),
            [size as u32, 0, 0, 0],
        );

        let response = kernel_service_call(&request)?;

        Ok(Self {
            handle: SharedBufferHandle::from_raw(response.args[0]),
            base: response.args[1] as usize,
            size: response.args[2] as usize,
        })
    }

    pub fn map(handle: SharedBufferHandle) -> Result<Self, SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Memory as u16,
                MemoryOp::SharedBufferMap as u16,
            ),
            [handle.raw(), 0, 0, 0],
        );

        let response = kernel_service_call(&request)?;

        Ok(Self {
            handle: SharedBufferHandle::from_raw(response.args[0]),
            base: response.args[1] as usize,
            size: response.args[2] as usize,
        })
    }

    pub fn handle(&self) -> SharedBufferHandle {
        self.handle
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.base as *const u8
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.base as *mut u8
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.size) }
    }

    pub fn unmap(self) -> Result<(), SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Memory as u16,
                MemoryOp::SharedBufferUnmap as u16,
            ),
            [self.handle.raw(), 0, 0, 0],
        );

        kernel_service_call(&request)?;

        Ok(())
    }

    pub fn free(self) -> Result<(), SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Memory as u16,
                MemoryOp::SharedBufferFree as u16,
            ),
            [self.handle.raw(), 0, 0, 0],
        );

        kernel_service_call(&request)?;

        Ok(())
    }
}
