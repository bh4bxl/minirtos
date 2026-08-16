#![no_std]
#![no_main]

mod early_init;

use cortex_m_rt::entry;
use defmt_rtt as _;
use minirtos_kernel::{KernelConfig, sys, task};
use panic_probe as _;

#[entry]
fn main() -> ! {
    defmt::info!(
        "miniRTOS {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );

    minirtos_kernel::init_heap();

    let mut config = KernelConfig::new();

    match early_init::early_init(&mut config) {
        Err(e) => {
            defmt::error!("Error: {:?}", e as u16);
            panic!("early init failed");
        }
        Ok(()) => defmt::info!("Board {} early initialized.", env!("CARGO_PKG_NAME"),),
    }

    match minirtos_kernel::init(&config) {
        Err(e) => {
            defmt::error!("Error: {:?}", e as u16);
            panic!("early init failed");
        }
        Ok(()) => defmt::info!("Kernel start."),
    }

    let _task = task::Task::new(default0)
        .stack_size(1024)
        .priority(task::Priority(100))
        .spawn()
        .unwrap();
    let _task = task::Task::new(default1)
        .stack_size(1024)
        .priority(task::Priority(100))
        .spawn()
        .unwrap();

    minirtos_kernel::start();
}

// Test Task
extern "C" fn default0(_arg: *mut ()) {
    let mut i = 0;
    loop {
        defmt::info!("task 0: {}", i);
        sys::sleep_ms(1000);
        i = i + 1;
    }
}

extern "C" fn default1(_arg: *mut ()) {
    for i in 0..10 {
        defmt::info!("task 1: {}", i);
        sys::sleep_ms(1500);
    }
    defmt::info!("task 1 exit");
}
