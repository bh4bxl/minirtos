use super::{SyscallId, syscall, syscall_noreturn, syscall_u64};

/// Voluntarily yield the CPU to the next ready task.
pub fn yield_now() {
    let _ = syscall::<{ SyscallId::Yield as u8 }>(&[]);
}

/// User application exit
pub fn exit() -> ! {
    syscall_noreturn::<{ SyscallId::Exit as u8 }>()
}

// ===== Clock =====
pub fn get_tick() -> u64 {
    syscall_u64::<{ SyscallId::GetTick as u8 }>(&[])
}

/// Sleep for `ms` milliseconds.
pub fn sleep_ms(ms: u32) {
    syscall::<{ SyscallId::Sleep as u8 }>(&[ms]);
}
