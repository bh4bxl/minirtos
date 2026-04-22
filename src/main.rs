#![no_std]
#![no_main]

use cortex_m::asm;
use defmt_rtt as _;
use panic_probe as _;

use crate::{
    bsp::board_init,
    sys::{cpu::start_first_task, scheduler, task::Priority},
};
use rp235x_hal as hal;

mod bsp;
mod drivers;
mod sys;

extern "C" fn task1_entry(_: *mut ()) -> ! {
    let mut cnt = 0u32;
    loop {
        cnt += 1;
        defmt::info!("task1 running {}", cnt);

        for _ in 0..20_000_000 {
            asm::nop();
        }
    }
}

extern "C" fn task2_entry(_: *mut ()) -> ! {
    let mut cnt = u32::MAX;
    loop {
        cnt -= 1;
        defmt::info!("task2 running {}", cnt);

        for _ in 0..40_000_000 {
            asm::nop();
        }
    }
}

#[hal::entry]
fn main() -> ! {
    defmt::info!("MINI RTOS");

    match board_init() {
        Err(e) => defmt::error!("Error: {}", e),
        Ok(()) => defmt::info!("Board {} initialized.", sys::board::board().board_name()),
    }

    println!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    scheduler::scheduler()
        .add_task(task1_entry, core::ptr::null_mut(), Priority(255), "task1")
        .unwrap();

    scheduler::scheduler()
        .add_task(task2_entry, core::ptr::null_mut(), Priority(255), "task2")
        .unwrap();

    unsafe {
        start_first_task();
    }
}
