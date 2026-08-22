use crate::{SysError, arch, sched, timer};

use super::{SyscallId, SyscallResult};

#[repr(u32)]
#[derive(Clone, Copy)]
pub(crate) enum TaskOp {
    Yield = 0,
    Exit = 1,
    GetTick = 2,
    Sleep = 3,
}

impl TryFrom<u32> for TaskOp {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Yield),
            1 => Ok(Self::Exit),
            2 => Ok(Self::GetTick),
            3 => Ok(Self::Sleep),
            _ => Err(()),
        }
    }
}

pub(crate) fn task_dispatch(op: u32, args: &[u32]) -> SyscallResult {
    let Ok(task_op) = TaskOp::try_from(op) else {
        return SyscallResult::Error(SysError::NotSupported);
    };

    match task_op {
        TaskOp::Yield => {
            arch::request_context_switch();
            SyscallResult::None
        }
        TaskOp::Exit => {
            sched::terminate_current_task();
            arch::request_context_switch();
            SyscallResult::None
        }
        TaskOp::GetTick => {
            let tick = timer::get_sys_tick();
            SyscallResult::U64(tick)
        }
        TaskOp::Sleep => {
            let ms = args[0];
            sched::sleep_current_task(ms);
            arch::request_context_switch();
            SyscallResult::None
        }
    }
}

//
// Syscalls
//

/// Voluntarily yield the CPU to the next ready task.
pub fn yield_now() {
    let _ = arch::syscall::<{ SyscallId::Task as u8 }>(&[TaskOp::Yield as u32]);
}

/// User application exit
pub fn exit() -> ! {
    arch::syscall_noreturn::<{ SyscallId::Task as u8 }>(&[TaskOp::Exit as u32])
}

/// Get system ticks
pub fn get_tick() -> u64 {
    arch::syscall_u64::<{ SyscallId::Task as u8 }>(&[TaskOp::GetTick as u32])
}

/// Sleep for `ms` milliseconds.
pub fn sleep_ms(ms: u32) {
    arch::syscall::<{ SyscallId::Task as u8 }>(&[TaskOp::Sleep as u32, ms]);
}
