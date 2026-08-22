use crate::{
    SysError,
    arch::{self, syscall},
    sched,
    synchronization::critical_section,
};

use super::{SyscallId, SyscallResult};

mod registry;

pub mod event;
pub mod mutex;
pub mod semaphore;

use registry::{SYNC_REGISTRY, SyncHandle};

#[repr(u32)]
pub(super) enum SyncOp {
    CreateSemaphore = 0,
    SemaphoreAcquire = 1,
    SemaphoreTryAcquire = 2,
    SemaphoreRelease = 3,

    CreateMutex = 10,
    MutexLock = 11,
    MutexUnlock = 12,

    CreateEvent = 20,
    EventWait = 21,
    EventSignal = 22,
    EventIsSignaled = 23,

    Destroy = u32::MAX,
}

impl TryFrom<u32> for SyncOp {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CreateSemaphore),
            1 => Ok(Self::SemaphoreAcquire),
            2 => Ok(Self::SemaphoreTryAcquire),
            3 => Ok(Self::SemaphoreRelease),
            10 => Ok(Self::CreateMutex),
            11 => Ok(Self::MutexLock),
            12 => Ok(Self::MutexUnlock),
            20 => Ok(Self::CreateEvent),
            21 => Ok(Self::EventWait),
            22 => Ok(Self::EventSignal),
            23 => Ok(Self::EventIsSignaled),
            u32::MAX => Ok(Self::Destroy),
            _ => Err(()),
        }
    }
}

pub(crate) fn sync_dispatch(op: u32, args: &[u32]) -> SyscallResult {
    let Ok(sync_op) = SyncOp::try_from(op) else {
        return SyscallResult::Error(SysError::NotSupported);
    };

    let res = match sync_op {
        // Semaphore
        SyncOp::CreateSemaphore => {
            let initial = args[0] as isize;
            let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

            let handle = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| registry.create_semaphore(owner, initial))
            });

            match handle {
                Some(handle) => handle.0,
                None => return SyscallResult::Error(SysError::NoResource),
            }
        }
        SyncOp::SemaphoreAcquire => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(sem) = registry.semaphore(handle) else {
                        return false;
                    };

                    sem.acquire_cs(cs);

                    true
                })
            });

            if !ok {
                return SyscallResult::Error(SysError::NotFound);
            }

            0
        }
        SyncOp::SemaphoreTryAcquire => {
            let handle = SyncHandle::from_raw(args[0]);

            let acquired = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    registry.semaphore(handle).map(|sem| sem.try_acquire_cs(cs))
                })
            });

            match acquired {
                Some(acquired) => acquired as u32,
                None => return SyscallResult::Error(SysError::NotFound),
            }
        }
        SyncOp::SemaphoreRelease => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(sem) = registry.semaphore(handle) else {
                        return false;
                    };

                    sem.release_cs(cs);

                    true
                })
            });

            if !ok {
                return SyscallResult::Error(SysError::NotFound);
            }

            0
        }
        // Mutex
        SyncOp::CreateMutex => {
            let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

            let handle = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| registry.create_mutex(owner))
            });

            match handle {
                Some(handle) => handle.0,
                None => return SyscallResult::Error(SysError::NoResource),
            }
        }
        SyncOp::MutexLock => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(mutex) = registry.mutex(handle) else {
                        return false;
                    };

                    mutex.lock_cs(cs);
                    true
                })
            });

            if !ok {
                return SyscallResult::Error(SysError::NotFound);
            }

            0
        }
        SyncOp::MutexUnlock => {
            let handle = SyncHandle::from_raw(args[0]);

            let need_reschedule = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    registry.mutex(handle).map(|mutex| mutex.unlock_cs(cs))
                })
            });

            let Some(need_reschedule) = need_reschedule else {
                return SyscallResult::Error(SysError::NotFound);
            };

            if need_reschedule {
                arch::request_context_switch();
            }

            0
        }
        // Event
        SyncOp::CreateEvent => {
            let signaled = args[0] != 0;
            let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

            let handle = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| registry.create_event(owner, signaled))
            });

            match handle {
                Some(handle) => handle.0,
                None => return SyscallResult::Error(SysError::NoResource),
            }
        }
        SyncOp::EventWait => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(event) = registry.event(handle) else {
                        return false;
                    };

                    event.wait_cs(cs);
                    true
                })
            });

            if !ok {
                return SyscallResult::Error(SysError::NotFound);
            }

            0
        }
        SyncOp::EventSignal => {
            let handle = SyncHandle::from_raw(args[0]);

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    let Some(event) = registry.event(handle) else {
                        return false;
                    };

                    event.signal_cs(cs);
                    true
                })
            });

            if !ok {
                return SyscallResult::Error(SysError::NotFound);
            }

            0
        }
        SyncOp::EventIsSignaled => {
            let handle = SyncHandle::from_raw(args[0]);

            let signaled = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| {
                    registry.event(handle).map(|event| event.is_signaled_cs(cs))
                })
            });

            match signaled {
                Some(signaled) => signaled as u32,
                None => return SyscallResult::Error(SysError::NotFound),
            }
        }
        // Destroy
        SyncOp::Destroy => {
            let handle = SyncHandle::from_raw(args[0]);

            let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

            let ok = critical_section(|cs| {
                SYNC_REGISTRY.lock(cs, |registry| registry.destroy(owner, handle))
            });

            if !ok {
                return SyscallResult::Error(SysError::NotFound);
            }

            0
        }
    };

    SyscallResult::U32(res)
}

//
// Syscall
//

fn sync_destroy(handle: &SyncHandle) -> Result<(), SysError> {
    let ret = syscall::<{ SyscallId::Sync as u8 }>(&[SyncOp::Destroy as u32, handle.0]) as i32;

    if ret >= 0 {
        Ok(())
    } else {
        Err(SysError::try_from(ret).unwrap_or(SysError::InvalidState))
    }
}
