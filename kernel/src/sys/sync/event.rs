use crate::{SysError, arch::syscall};

use super::{
    super::{SyscallId, syscall_result},
    SyncHandle, SyncOp,
};

pub struct Event {
    handle: SyncHandle,
}

impl Event {
    pub fn new(initially_signaled: bool) -> Result<Self, SysError> {
        Ok(Self {
            handle: create_event(initially_signaled)?,
        })
    }

    pub fn unsignaled() -> Result<Self, SysError> {
        Self::new(false)
    }

    pub fn signaled() -> Result<Self, SysError> {
        Self::new(true)
    }

    pub fn wait(&self) -> Result<(), SysError> {
        event_wait(&self.handle)
    }

    pub fn signal(&self) -> Result<(), SysError> {
        event_signal(&self.handle)
    }

    pub fn is_signaled(self) -> Result<bool, SysError> {
        event_is_signaled(&self.handle)
    }

    pub fn destroy(self) -> Result<(), SysError> {
        super::sync_destroy(&self.handle)
    }
}

//
// Syscalls
//

fn create_event(initially_signaled: bool) -> Result<SyncHandle, SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[
        SyncOp::CreateEvent as u32,
        initially_signaled as u32,
    ]);

    Ok(SyncHandle::from_raw(ret))
}

fn event_wait(handle: &SyncHandle) -> Result<(), SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::EventWait as u32, handle.0]);

    syscall_result(ret)?;

    Ok(())
}

fn event_signal(handle: &SyncHandle) -> Result<(), SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::EventSignal as u32, handle.0]);

    syscall_result(ret)?;

    Ok(())
}

fn event_is_signaled(handle: &SyncHandle) -> Result<bool, SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::EventIsSignaled as u32, handle.0]);

    if let Ok(err) = SysError::try_from(ret as i32) {
        return Err(err);
    }

    Ok(ret != 0)
}
