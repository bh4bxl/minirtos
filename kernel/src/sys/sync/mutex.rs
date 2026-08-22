use core::marker::PhantomData;

use crate::{SysError, arch::syscall};

use super::{super::SyscallId, SyncHandle, SyncOp};

pub struct Mutex {
    handle: SyncHandle,
}

pub struct MutexGuard<'a> {
    mutex: &'a Mutex,
    locked: bool,

    // Mutex ownership belongs to the current task.
    // Don't allow the guard to be moved to another task/thread.
    _not_send: PhantomData<*mut ()>,
}

impl Mutex {
    pub fn new() -> Result<Self, SysError> {
        Ok(Self {
            handle: mutex_create()?,
        })
    }

    pub fn lock(&self) -> Result<MutexGuard<'_>, SysError> {
        mutex_lock(&self.handle)?;

        Ok(MutexGuard {
            mutex: self,
            locked: true,
            _not_send: PhantomData,
        })
    }

    pub fn destroy(self) -> Result<(), SysError> {
        super::sync_destroy(&self.handle)
    }
}

impl MutexGuard<'_> {
    pub fn unlock(mut self) -> Result<(), SysError> {
        mutex_unlock(&self.mutex.handle)?;
        self.locked = false;
        Ok(())
    }
}

impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        if self.locked {
            let _ = mutex_unlock(&self.mutex.handle);
        }
    }
}

//
// Syscalls
//

fn mutex_create() -> Result<SyncHandle, SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::CreateMutex as u32]) as i32;

    if ret < 0 {
        return Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState));
    }

    Ok(SyncHandle::from_raw(ret as u32))
}

fn mutex_lock(handle: &SyncHandle) -> Result<(), SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::MutexLock as u32, handle.0]) as i32;

    if ret >= 0 {
        Ok(())
    } else {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    }
}

fn mutex_unlock(handle: &SyncHandle) -> Result<(), SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::MutexUnlock as u32, handle.0]) as i32;

    if ret >= 0 {
        Ok(())
    } else {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    }
}
