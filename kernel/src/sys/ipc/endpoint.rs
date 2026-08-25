use minirtos_abi::{
    EndpointHandle, IpcOp, IpcRecvArgs, IpcSendArgs, MessageData, ReceivedMessage, SysError,
    SyscallId, UserMutPtr, UserPtr,
};

use crate::arch::syscall;

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

    pub fn try_recv(&self) -> Result<Option<ReceivedMessage>, SysError> {
        let mut message = ReceivedMessage::default();

        let args = IpcRecvArgs {
            endpoint: self.handle,
            message: UserMutPtr::from_raw(&mut message as *mut ReceivedMessage as u32),
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

    pub fn recv(&self) -> Result<ReceivedMessage, SysError> {
        let mut message = ReceivedMessage::default();

        let args = IpcRecvArgs {
            endpoint: self.handle,
            message: UserMutPtr::from_raw(&mut message as *mut ReceivedMessage as u32),
        };

        let ret = syscall::<{ SyscallId::Ipc as u8 }>(&[
            IpcOp::Recv as u32,
            &args as *const IpcRecvArgs as u32,
        ]) as i32;

        if ret < 0 {
            return Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState));
        }

        Ok(message)
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
