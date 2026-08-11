#![allow(dead_code)]
use core::fmt;

use crate::sys::{
    console::{
        interface::{Read, Write},
        syscall_console::SyscallConsole,
    },
    task::Privilege,
};

use super::{
    SysError,
    scheduler::{self, WaitTaskResult},
    synchronization::{critical_section, interface::Mutex},
    task::{Priority, TaskEntry, TaskId},
};

#[repr(u8)]
#[derive(Clone, Copy)]
pub(super) enum Syscall {
    Start = 0,
    Yield = 1,
    Exit = 2,
    Sleep = 3,
    GetTick = 4,
    Write = 5,
    TryReadChar = 6,
}

impl TryFrom<u8> for Syscall {
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
            _ => Err(()),
        }
    }
}

/// Voluntarily yield the CPU to the next ready task.
pub fn yield_now() {
    unsafe {
        core::arch::asm! {
            "svc {svc}",
            svc = const Syscall::Yield as u8,
            options(nomem, nostack, preserves_flags),
        }
    }
}

/// User application exit
pub fn exit() -> ! {
    unsafe {
        core::arch::asm! {
            "svc {svc}",
            svc = const Syscall::Exit as u8,
            options(noreturn),
        }
    }
}

// ===== Clock =====
pub fn get_tick() -> u64 {
    let low: u32;
    let high: u32;

    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const Syscall::GetTick as u8,
            lateout("r0") low,
            lateout("r1") high,
            options(nostack),
        );
    }

    ((high as u64) << 32) | low as u64
}

/// Sleep for `ms` milliseconds.
pub fn sleep_ms(ms: u32) {
    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const Syscall::Sleep as u8,
            in("r0") ms,
            options(nostack),
        );
    }
}

/// Write string to console
pub fn write(buf: *const u8, len: usize) {
    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const Syscall::Write as u8,
            in("r0") buf,
            in("r1") len,
            options(nostack),
        );
    }
}

/// Try read a char
pub fn try_read_char() -> Option<char> {
    let ret: u32;

    unsafe {
        core::arch::asm!(
            "svc {svc}",
            svc = const Syscall::TryReadChar as u8,
            lateout("r0") ret,
            options(nostack),
        );
    }

    if ret == u32::MAX {
        None
    } else {
        char::from_u32(ret)
    }
}

/// Read line
pub fn read_line<'a>(buf: &'a mut [u8]) -> &'a str {
    SyscallConsole::new().read_line(buf)
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

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    SyscallConsole::new().write_fmt(args).unwrap();
}

/// Prints without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::sys::syscall::_print(format_args!($($arg)*)));
}

/// Prints with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\r\n")
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\r\n"))
    };
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\r\n"), $($arg)*));
}
