use crate::{SysError, arch::syscall};

use super::{super::SyscallId, SyncHandle, SyncOp};

pub struct Semaphore {
    handle: SyncHandle,
}

impl Semaphore {
    pub fn new(initial: u32) -> Result<Self, SysError> {
        Ok(Self {
            handle: create_semaphore(initial)?,
        })
    }

    pub fn acquire(&self) -> Result<(), SysError> {
        semaphore_acquire(&self.handle)
    }

    pub fn try_acquire(&self) -> Result<bool, SysError> {
        semaphore_try_acquire(&self.handle)
    }

    pub fn release(&self) -> Result<(), SysError> {
        semaphore_release(&self.handle)
    }

    pub fn destroy(self) -> Result<(), SysError> {
        super::sync_destroy(&self.handle)
    }
}

//
// Syscalls
//

fn create_semaphore(initial: u32) -> Result<SyncHandle, SysError> {
    let ret = syscall_result(syscall::<{ SyscallId::Sync as u8 }>(&[
        SyncOp::CreateSemaphore as u32,
        initial,
    ]))?;

    Ok(SyncHandle::from_raw(ret))
}

fn semaphore_acquire(handle: &SyncHandle) -> Result<(), SysError> {
    loop {
        let ret =
            syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::SemaphoreAcquire as u32, handle.0]);

        match syscall_result(ret) {
            Ok(_) => return Ok(()),

            Err(SysError::WouldBlock) => {
                // The kernel has blocked this task and pended PendSV.
                // After being woken, retry acquiring the semaphore.
                continue;
            }

            Err(err) => return Err(err),
        }
    }
}

fn semaphore_try_acquire(handle: &SyncHandle) -> Result<bool, SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::SemaphoreTryAcquire as u32, handle.0]);

    match syscall_result(ret) {
        Ok(_) => Ok(true),
        Err(SysError::WouldBlock) => Ok(false),
        Err(err) => Err(err),
    }
}

fn semaphore_release(handle: &SyncHandle) -> Result<(), SysError> {
    syscall_result(syscall::<{ SyscallId::Sync as u8 }>(&[
        SyncOp::SemaphoreRelease as u32,
        handle.0,
    ]))?;

    Ok(())
}

fn syscall_result(ret: u32) -> Result<u32, SysError> {
    let value = ret as i32;

    if value >= 0 {
        Ok(ret)
    } else {
        Err(SysError::try_from(value).unwrap_or(SysError::InvalidState))
    }
}
