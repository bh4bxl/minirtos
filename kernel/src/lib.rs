#![no_std]

#[cfg(feature = "defmt")]
pub use defmt::{debug as kdebug, error as kerror, info as kinfo, trace as ktrace, warn as kwarn};

extern crate alloc;

mod arch;
mod ipc;
mod logger;
mod memory;
mod sched;
mod service;
mod timer;

pub mod interface;
pub mod stdio;
pub mod synchronization;
pub mod sys;
pub mod task;

use minirtos_abi::{Priority, SysError};

pub use memory::{MemoryAccess, MemoryBlock, MemoryRegion, init_heap};

use crate::{service::kernel_service, synchronization::critical_section, sys::sleep_ms};

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

    arch::init_protection();

    critical_section(|cs| kernel_service().lock(cs, |service| service.init(cs)))?;

    sched::init()?;

    Ok(())
}

const INIT_TASK_STACK_SIZE: usize = 2048;

pub fn start() -> ! {
    let _init_task = task::Task::new(init_task)
        .stack_size(INIT_TASK_STACK_SIZE)
        .priority(Priority(10))
        .privilege(task::Privilege::Privileged)
        .name("init_task")
        .spawn()
        .unwrap();

    arch::start_first_task();
}

// Callbacks for special boards.
unsafe extern "Rust" {
    fn drivers_init() -> Result<(), SysError>;

    fn services_init() -> Result<(), SysError>;

    unsafe fn user_apps_init() -> Result<(), SysError>;

}

extern "C" fn init_task(_arg: *mut ()) {
    defmt::info!("init_task start");

    unsafe {
        drivers_init().unwrap();
    }

    sleep_ms(100);

    unsafe {
        services_init().unwrap();
    }

    sleep_ms(100);

    unsafe {
        user_apps_init().unwrap();
    }

    loop {
        sleep_ms(1000);
    }
}
