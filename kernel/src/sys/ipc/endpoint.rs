use minirtos_abi::{
    EndpointHandle, IpcOp, IpcReadArgs, IpcRecvArgs, IpcSendArgs, IpcWriteArgs, MessageData,
    ReceivedRequest, SysError, SyscallId, UserMutPtr, UserPtr,
};

use crate::{arch::syscall, task::TaskId};

pub struct Endpoint {
    handle: EndpointHandle,
}

impl Endpoint {
    pub fn create() -> Result<Self, SysError> {
        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[IpcOp::CreateEndpoint as u32]) as i32;

        if ret < 0 {
            return Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState));
        }

        Ok(Self {
            handle: EndpointHandle::from_raw(ret as u32),
        })
    }

    pub fn try_send(&self, message: &MessageData) -> Result<(), SysError> {
        let args = IpcSendArgs {
            endpoint: self.handle,
            message: UserPtr::from_raw(message as *const MessageData as u32),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::TrySend as u32,
            &args as *const IpcSendArgs as u32,
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }

    pub fn try_recv(&self) -> Result<Option<ReceivedRequest>, SysError> {
        let mut message = ReceivedRequest::default();

        let args = IpcRecvArgs {
            endpoint: self.handle,
            request: UserMutPtr::from_raw(&mut message as *mut ReceivedRequest as u32),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::TryRecv as u32,
            &args as *const IpcRecvArgs as u32,
        ]) as i32;

        if ret == SysError::WouldBlock as i32 {
            return Ok(None);
        }

        if ret < 0 {
            return Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState));
        }

        Ok(Some(message))
    }

    pub fn send(&self, message: &MessageData) -> Result<(), SysError> {
        let args = IpcSendArgs {
            endpoint: self.handle,
            message: UserPtr::from_raw(message as *const MessageData as u32),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Send as u32,
            &args as *const IpcSendArgs as u32,
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }

    pub fn recv(&self) -> Result<ReceivedRequest, SysError> {
        let mut request = ReceivedRequest::default();

        let args = IpcRecvArgs {
            endpoint: self.handle,
            request: UserMutPtr::from_raw(&mut request as *mut ReceivedRequest as u32),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Recv as u32,
            &args as *const IpcRecvArgs as u32,
        ]) as i32;

        if ret < 0 {
            return Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState));
        }

        Ok(request)
    }

    pub fn write(&self, op: u32, buf: &[u8]) -> Result<usize, SysError> {
        let args = IpcWriteArgs {
            endpoint: self.handle,
            op,
            ptr: UserPtr::from_raw(buf.as_ptr() as u32),
            len: buf.len(),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Write as u32,
            &args as *const IpcWriteArgs as u32,
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(ret as usize)
        }
    }

    pub fn read(&self, op: u32, buf: &mut [u8]) -> Result<usize, SysError> {
        let args = IpcReadArgs {
            endpoint: self.handle,
            op,
            ptr: UserMutPtr::from_raw(buf.as_mut_ptr() as u32),
            len: buf.len(),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Read as u32,
            &args as *const IpcReadArgs as u32,
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(ret as usize)
        }
    }

    pub fn complete(&self, sender: TaskId, res: Result<usize, SysError>) -> Result<(), SysError> {
        let val = match res {
            Ok(n) => n as i32,
            Err(err) => err as i32,
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Complete as u32,
            self.handle.raw(),
            sender.raw() as u32,
            val as u32,
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }

    pub fn destroy(self) -> Result<(), SysError> {
        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::DestroyEndpoint as u32,
            self.handle.raw(),
        ]) as i32;

        if ret < 0 {
            Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
        } else {
            Ok(())
        }
    }

    pub(crate) const fn handle(&self) -> EndpointHandle {
        self.handle
    }

    pub(crate) const fn from_handle(handle: EndpointHandle) -> Self {
        Self { handle }
    }
}
