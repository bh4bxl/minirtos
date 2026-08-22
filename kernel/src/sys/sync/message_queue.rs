use crate::{SysError, arch::syscall, sys};

use super::{super::SyscallId, SyncHandle, SyncOp};

pub struct MessageQueue {
    handle: SyncHandle,
}

impl MessageQueue {
    pub fn new() -> Result<Self, SysError> {
        Ok(Self {
            handle: message_queue_create()?,
        })
    }

    pub fn send(&self, msg: u32) -> Result<(), SysError> {
        message_queue_send(&self.handle, msg)
    }

    pub fn try_send(&self, msg: u32) -> Result<bool, SysError> {
        message_queue_try_send(&self.handle, msg)
    }

    pub fn recv(&self) -> Result<u32, SysError> {
        loop {
            match message_queue_recv(&self.handle) {
                Ok(msg) => return Ok(msg),
                Err(SysError::WouldBlock) => sys::yield_now(),
                Err(err) => return Err(err),
            }
        }
    }

    pub fn destroy(self) -> Result<(), SysError> {
        super::sync_destroy(&self.handle)
    }
}

//
// Syscalls
//

fn message_queue_create() -> Result<SyncHandle, SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::CreateMessageQueue as u32]) as i32;

    if ret < 0 {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    } else {
        Ok(SyncHandle::from_raw(ret as u32))
    }
}

fn message_queue_send(handle: &SyncHandle, msg: u32) -> Result<(), SysError> {
    let ret =
        syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::MessageQueueSend as u32, handle.0, msg])
            as i32;

    if ret < 0 {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    } else {
        Ok(())
    }
}

fn message_queue_try_send(handle: &SyncHandle, msg: u32) -> Result<bool, SysError> {
    let ret =
        syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::MessageQueueTrySend as u32, handle.0, msg])
            as i32;

    if ret < 0 {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    } else {
        Ok(ret != 0)
    }
}

fn message_queue_recv(handle: &SyncHandle) -> Result<u32, SysError> {
    let ret =
        syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::MessageQueueRecv as u32, handle.0]) as i32;

    if ret < 0 {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    } else {
        Ok(ret as u32)
    }
}
