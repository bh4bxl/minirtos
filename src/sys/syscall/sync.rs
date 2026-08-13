use crate::sys::{
    SysError,
    scheduler::scheduler,
    synchronization::{CriticalSectionLock, critical_section},
};

use super::{
    super::super::sys::{sync, task::TaskId},
    SyscallId, syscall, syscall_result,
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncHandle(u32);

impl SyncHandle {
    pub const INVALID: Self = Self(u32::MAX);

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

enum SyncObject {
    Semaphore(sync::Semaphore),
}

struct SyncEntry {
    owner: TaskId,
    object: SyncObject,
}

const MAX_SYNC_OBJECTS: usize = 16;

pub struct SyncRegistry {
    entries: [Option<SyncEntry>; MAX_SYNC_OBJECTS],
}

impl SyncRegistry {
    pub const fn new() -> Self {
        Self {
            entries: [const { None }; MAX_SYNC_OBJECTS],
        }
    }

    fn create_semaphore(&mut self, owner: TaskId, initial: isize) -> Option<SyncHandle> {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(SyncEntry {
                    owner,
                    object: SyncObject::Semaphore(sync::Semaphore::new(initial)),
                });

                return Some(SyncHandle::from_raw(index as u32));
            }
        }

        None
    }

    fn semaphore(&self, handle: SyncHandle) -> Option<&sync::Semaphore> {
        let entry = self.entries.get(handle.0 as usize)?.as_ref()?;

        match &entry.object {
            SyncObject::Semaphore(sem) => Some(sem),
        }
    }

    fn destroy(&mut self, owner: TaskId, handle: SyncHandle) -> bool {
        let Some(entry) = self.entries.get_mut(handle.0 as usize) else {
            return false;
        };

        let Some(obj) = entry.as_ref() else {
            return false;
        };

        if obj.owner != owner {
            return false;
        }

        *entry = None;
        true
    }
}

static SYNC_REGISTRY: CriticalSectionLock<SyncRegistry> =
    CriticalSectionLock::new(SyncRegistry::new());

#[repr(u32)]
pub enum SyncOp {
    CreateSemaphore = 0,
    SemaphoreWait = 1,
    SemaphoreTryWait = 2,
    SemaphoreSignal = 3,

    CreateMutex = 4,
    MutexLock = 5,
    MutexUnlock = 6,

    CreateEvent = 7,
    EventWait = 8,
    EventSet = 9,

    Destroy = 10,
}

impl TryFrom<u32> for SyncOp {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CreateSemaphore),
            1 => Ok(Self::SemaphoreWait),
            2 => Ok(Self::SemaphoreTryWait),
            3 => Ok(Self::SemaphoreSignal),
            4 => Ok(Self::CreateMutex),
            5 => Ok(Self::MutexLock),
            6 => Ok(Self::MutexUnlock),
            7 => Ok(Self::CreateEvent),
            8 => Ok(Self::EventWait),
            9 => Ok(Self::EventSet),
            10 => Ok(Self::Destroy),
            _ => Err(()),
        }
    }
}

// Syscalls for Semaphore

fn semaphore_create(initial: u32) -> Result<SyncHandle, SysError> {
    let ret = syscall_result(syscall::<{ SyscallId::Sync as u8 }>(&[
        SyncOp::CreateSemaphore as u32,
        initial,
    ]))?;

    Ok(SyncHandle::from_raw(ret))
}

fn semaphore_wait(handle: &SyncHandle) -> Result<(), SysError> {
    loop {
        let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::SemaphoreWait as u32, handle.0]);

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

pub fn semaphore_try_wait(handle: &SyncHandle) -> Result<bool, SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::SemaphoreTryWait as u32, handle.0]);

    match syscall_result(ret) {
        Ok(_) => Ok(true),
        Err(SysError::WouldBlock) => Ok(false),
        Err(err) => Err(err),
    }
}

pub fn semaphore_signal(handle: &SyncHandle) -> Result<(), SysError> {
    syscall_result(syscall::<{ SyscallId::Sync as u8 }>(&[
        SyncOp::SemaphoreSignal as u32,
        handle.0,
    ]))?;

    Ok(())
}

pub fn sync_destroy(handle: &SyncHandle) -> Result<(), SysError> {
    syscall_result(syscall::<{ SyscallId::Sync as u8 }>(&[
        SyncOp::Destroy as u32,
        handle.0,
    ]))?;

    Ok(())
}

pub(crate) fn sync_dispatch(op: u32, args: &[u32]) -> u32 {
    match SyncOp::try_from(op) {
        Ok(SyncOp::CreateSemaphore) => {
            let initial = args[0] as isize;
            let owner = critical_section(|cs| scheduler().current_task_id(cs));

            critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    registry
                        .create_semaphore(owner, initial)
                        .unwrap_or(SyncHandle::INVALID)
                        .0
                })
            })
        }
        Ok(SyncOp::SemaphoreWait) => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(sem) = registry.semaphore(handle) else {
                        return false;
                    };

                    sem.wait_cs(cs);

                    true
                })
            });

            if ok { 0 } else { SyncHandle::INVALID.0 }
        }
        Ok(SyncOp::SemaphoreTryWait) => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |regsitry| {
                    let Some(sem) = regsitry.semaphore(handle) else {
                        return false;
                    };

                    sem.try_wait_cs(cs)
                })
            });

            if ok { 0 } else { SyncHandle::INVALID.0 }
        }
        Ok(SyncOp::SemaphoreSignal) => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(sem) = registry.semaphore(handle) else {
                        return false;
                    };

                    sem.signal_cs(cs);

                    true
                })
            });

            if ok { 0 } else { SyncHandle::INVALID.0 }
        }
        Ok(SyncOp::Destroy) => {
            let handle = SyncHandle::from_raw(args[0]);

            let owner = critical_section(|cs| scheduler().current_task_id(cs));

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| registry.destroy(owner, handle))
            });

            if ok { 0 } else { SyncHandle::INVALID.0 }
        }
        _ => u32::MAX,
    }
}

pub struct Semaphore {
    handle: SyncHandle,
}

impl Semaphore {
    pub fn new(initial: u32) -> Result<Self, SysError> {
        Ok(Self {
            handle: semaphore_create(initial)?,
        })
    }

    pub fn wait(&self) -> Result<(), SysError> {
        semaphore_wait(&self.handle)
    }

    pub fn try_wait(&self) -> Result<bool, SysError> {
        semaphore_try_wait(&self.handle)
    }

    pub fn signal(&self) -> Result<(), SysError> {
        semaphore_signal(&self.handle)
    }

    pub fn destroy(self) -> Result<(), SysError> {
        sync_destroy(&self.handle)
    }
}
