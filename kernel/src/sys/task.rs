use crate::arch;

use super::SyscallId;

/// Voluntarily yield the CPU to the next ready task.
pub fn yield_now() {
    let _ = arch::syscall::<{ SyscallId::Yield as u8 }>(&[]);
}

/// User application exit
pub fn exit() -> ! {
    arch::syscall_noreturn::<{ SyscallId::Exit as u8 }>()
}

/// Get system ticks
pub fn get_tick() -> u64 {
    arch::syscall_u64::<{ SyscallId::GetTick as u8 }>(&[])
}

/// Sleep for `ms` milliseconds.
pub fn sleep_ms(ms: u32) {
    arch::syscall::<{ SyscallId::Sleep as u8 }>(&[ms]);
}
