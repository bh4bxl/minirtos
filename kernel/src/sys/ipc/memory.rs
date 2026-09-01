use minirtos_abi::{
    IpcOp, IpcReadArgs, IpcSendArgs, MessageData, SharedBufferHandle, SharedBufferInfo, SysError,
    SyscallId, UserMutPtr, UserPtr,
};

use crate::{
    arch::syscall,
    service::{KernelServiceClass, MemoryOp, kernel_service, make_op},
    synchronization::critical_section,
};

use super::super::syscall_result;

pub struct SharedBuffer {
    handle: SharedBufferHandle,
    ptr: *mut u8,
    size: usize,
}

impl SharedBuffer {
    pub fn alloc(size: usize) -> Result<Self, SysError> {
        let mut info = SharedBufferInfo {
            handle: SharedBufferHandle::from_raw(0),
            addr: 0,
            size: size as u32,
        };

        let op = make_op(
            KernelServiceClass::Memory as u16,
            MemoryOp::SharedBufferAlloc as u16,
        );

        let endpoint =
            critical_section(|cs| kernel_service().lock(cs, |service| service.endpoint()))?;

        let args = IpcReadArgs {
            endpoint,
            op,
            ptr: UserMutPtr::from_raw((&mut info as *mut SharedBufferInfo) as u32),
            len: core::mem::size_of::<SharedBufferInfo>(),
        };

        let result = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Read as u32,
            (&args as *const IpcReadArgs) as u32,
        ]);

        syscall_result(result)?;

        Ok(Self {
            handle: info.handle,
            ptr: info.addr as *mut u8,
            size: info.size as usize,
        })
    }

    pub fn map(handle: SharedBufferHandle) -> Result<Self, SysError> {
        let mut info = SharedBufferInfo {
            handle,
            addr: 0,
            size: 0,
        };

        let op = make_op(
            KernelServiceClass::Memory as u16,
            MemoryOp::SharedBufferMap as u16,
        );

        let endpoint =
            critical_section(|cs| kernel_service().lock(cs, |service| service.endpoint()))?;

        let args = IpcReadArgs {
            endpoint,
            op,
            ptr: UserMutPtr::from_raw((&mut info as *mut SharedBufferInfo) as u32),
            len: core::mem::size_of::<SharedBufferInfo>(),
        };

        let result = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Read as u32,
            (&args as *const IpcReadArgs) as u32,
        ]);

        syscall_result(result)?;

        Ok(Self {
            handle: info.handle,
            ptr: info.addr as *mut u8,
            size: info.size as usize,
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
        self.ptr
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.size) }
    }

    pub fn unmap(self) -> Result<(), SysError> {
        let op = make_op(
            KernelServiceClass::Memory as u16,
            MemoryOp::SharedBufferUnmap as u16,
        );

        let endpoint =
            critical_section(|cs| kernel_service().lock(cs, |service| service.endpoint()))?;

        let message = MessageData {
            op,
            args: [self.handle.raw(), 0, 0, 0],
        };

        let args = IpcSendArgs {
            endpoint,
            message: UserPtr::from_raw((&message as *const MessageData) as u32),
        };

        let result = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Send as u32,
            (&args as *const IpcSendArgs) as u32,
        ]);

        syscall_result(result)?;

        Ok(())
    }

    pub fn free(self) -> Result<(), SysError> {
        let message = MessageData {
            op: make_op(
                KernelServiceClass::Memory as u16,
                MemoryOp::SharedBufferFree as u16,
            ),
            args: [self.handle.raw(), 0, 0, 0],
        };

        let endpoint =
            critical_section(|cs| kernel_service().lock(cs, |service| service.endpoint()))?;

        let args = IpcSendArgs {
            endpoint,
            message: UserPtr::from_raw((&message as *const MessageData) as u32),
        };

        let result = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Send as u32,
            (&args as *const IpcSendArgs) as u32,
        ]);

        syscall_result(result)?;

        Ok(())
    }
}
