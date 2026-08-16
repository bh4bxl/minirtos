use crate::SysError;

mod task;

pub use task::{exit, get_tick, sleep_ms, yield_now};

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

fn syscall_result(ret: u32) -> Result<u32, SysError> {
    let value = ret as i32;

    if value >= 0 {
        Ok(ret)
    } else {
        Err(SysError::try_from(value).unwrap_or(SysError::InvalidState))
    }
}
