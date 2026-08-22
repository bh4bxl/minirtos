#![no_std]

#[cfg(feature = "defmt")]
pub use defmt::{debug as kdebug, error as kerror, info as kinfo, trace as ktrace, warn as kwarn};

extern crate alloc;

mod arch;
mod error;
mod ipc;
mod logger;
mod memory;
mod sched;
mod synchronization;
pub mod sys;
pub mod task;
mod timer;

pub use error::SysError;
pub use memory::init_heap;

pub struct KernelConfig {
    pub core_clock_hz: u32,
    pub tick_hz: u32,
}

impl KernelConfig {
    pub fn new() -> Self {
        Self {
            core_clock_hz: 0,
            tick_hz: 1000,
        }
    }
}

pub fn init(config: &KernelConfig) -> Result<(), SysError> {
    crate::kinfo!("Kernel initializing");

    timer::init(config.tick_hz);

    arch::init(config.core_clock_hz, config.tick_hz);

    sched::init();

    Ok(())
}

pub fn start() -> ! {
    arch::start_first_task();
}
