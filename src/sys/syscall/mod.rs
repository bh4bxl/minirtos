// #![allow(dead_code)]

use crate::sys::task::Privilege;

use super::{
    SysError,
    scheduler::{self, WaitTaskResult},
    synchronization::{critical_section, interface::Mutex},
    task::{Priority, TaskEntry, TaskId},
};

mod console_io;
mod sync;
mod tasks;

#[allow(unused_imports)]
pub use console_io::{_print, read_line, try_read_char};
pub use sync::Semaphore;
pub(crate) use sync::sync_dispatch;
#[allow(unused_imports)]
pub use tasks::{exit, get_tick, sleep_ms, yield_now};

#[repr(u8)]
#[derive(Clone, Copy)]
pub(super) enum SyscallId {
    Start = 0,
    Yield = 1,
    Exit = 2,
    Sleep = 3,
    GetTick = 4,
    Write = 5,
    TryReadChar = 6,
    Sync = 7,
}

impl TryFrom<u8> for SyscallId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Start),
            1 => Ok(Self::Yield),
            2 => Ok(Self::Exit),
            3 => Ok(Self::Sleep),
            4 => Ok(Self::GetTick),
            5 => Ok(Self::Write),
            6 => Ok(Self::TryReadChar),
            7 => Ok(Self::Sync),
            _ => Err(()),
        }
    }
}

#[inline]
fn syscall<const ID: u8>(args: &[u32]) -> u32 {
    assert!(args.len() <= 4);

    let mut r0 = args.first().copied().unwrap_or(0);
    let r1 = args.get(1).copied().unwrap_or(0);
    let r2 = args.get(2).copied().unwrap_or(0);
    let r3 = args.get(3).copied().unwrap_or(0);

    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const ID,
            inlateout("r0") r0,
            in("r1") r1,
            in("r2") r2,
            in("r3") r3,
            options(nostack),
        );
    }

    r0
}

#[inline]
fn syscall_u64<const ID: u8>(args: &[u32]) -> u64 {
    assert!(args.len() <= 4);

    let mut r0 = args.get(0).copied().unwrap_or(0);
    let mut r1 = args.get(1).copied().unwrap_or(0);
    let r2 = args.get(2).copied().unwrap_or(0);
    let r3 = args.get(3).copied().unwrap_or(0);

    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const ID,
            inlateout("r0") r0,
            inlateout("r1") r1,
            in("r2") r2,
            in("r3") r3,
            options(nostack),
        );
    }

    ((r1 as u64) << 32) | r0 as u64
}

#[inline]
fn syscall_noreturn<const ID: u8>() -> ! {
    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const ID,
            options(noreturn),
        );
    }
}

fn syscall_result(ret: u32) -> Result<u32, SysError> {
    let value = ret as i32;

    if value >= 0 {
        Ok(ret)
    } else {
        Err(SysError::try_from(value).unwrap_or(SysError::InvalidState))
    }
}

#[allow(dead_code)]
/// Spawn a task
pub fn task_spawn(
    task_entry: TaskEntry,
    arg: *mut (),
    stack_words: usize,
    priority: Priority,
    privilege: Privilege,
    name: &'static str,
) -> Result<TaskId, SysError> {
    let stack = super::task::STACK_POOL.lock(|pool| pool.alloc_words(stack_words))?;

    match critical_section(|cs| {
        scheduler::scheduler().add_task(cs, task_entry, arg, stack, priority, privilege, name)
    }) {
        Ok(task_id) => Ok(task_id),

        Err((error, stack)) => {
            super::task::STACK_POOL.lock(|pool| {
                pool.free_words(stack);
            });

            Err(error)
        }
    }
}

pub fn task_wait(task_id: TaskId) -> Result<(), SysError> {
    let wait_result = critical_section(|cs| scheduler::scheduler().wait_task(cs, task_id))?;

    if matches!(wait_result, WaitTaskResult::Blocked) {
        super::arch::arm_cortex_m::trigger_pendsv();
    }

    let stack = critical_section(|cs| scheduler::scheduler().reap_task(cs, task_id))?;

    super::task::STACK_POOL.lock(|pool| {
        pool.free_words(stack);
    });

    Ok(())
}

#[allow(dead_code)]
/// End current task
pub fn task_exit() -> ! {
    super::task::exit_current_task();
}

pub fn stack_pool_total() -> usize {
    super::task::STACK_POOL.lock(|inner| inner.total())
}

pub fn stack_pool_used() -> usize {
    super::task::STACK_POOL.lock(|inner| inner.used())
}

pub fn stack_pool_free() -> usize {
    super::task::STACK_POOL.lock(|inner| inner.free())
}
