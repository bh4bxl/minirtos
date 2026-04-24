#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use crate::{
    bsp::board_init,
    sys::{
        arch::arm_cortex_m::start_first_task,
        device_driver::{self, DeviceType},
        syscall,
        task::Priority,
    },
};
use rp235x_hal as hal;

mod bsp;
mod drivers;
mod sys;

#[hal::entry]
fn main() -> ! {
    defmt::info!("MINI RTOS");

    match board_init() {
        Err(e) => defmt::error!("Error: {}", e),
        Ok(()) => defmt::info!("Board {} initialized.", sys::board::board().board_name()),
    }

    m_info!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    syscall::thread_create(idle_task, core::ptr::null_mut(), Priority(255), "idle").unwrap();

    syscall::thread_create(task1_entry, core::ptr::null_mut(), Priority(100), "task1").unwrap();

    syscall::thread_create(task2_entry, core::ptr::null_mut(), Priority(100), "task2").unwrap();

    unsafe {
        start_first_task();
    }
}

// Tasks

extern "C" fn idle_task(_: *mut ()) -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}

fn trigger_gpio(pin: u8) {
    if let Some(gpio) = device_driver::driver_manager().open_device(DeviceType::Gpio, 0) {
        let mut data = [pin, 0];
        if let Err(_x) = gpio.read(&mut data) {
            defmt::error!("GPIO read failed: {}", data[0]);
        }

        data[1] = if data[1] == 0 { 1 } else { 0 };

        if let Err(_x) = gpio.write(&data) {
            defmt::error!("GPIO write failed: {}", data[0]);
        }
    }
}

extern "C" fn task1_entry(_: *mut ()) -> ! {
    let mut cnt = 0u32;
    loop {
        cnt += 1;
        defmt::info!("task1 running {}", cnt);
        m_info!("task1 running {}", cnt);

        trigger_gpio(19);

        syscall::sleep_ms(1000);
    }
}

extern "C" fn task2_entry(_: *mut ()) -> ! {
    let mut cnt = u32::MAX;
    loop {
        cnt -= 1;
        defmt::info!("task2 running {}", cnt);
        m_info!("task2 running {}", cnt);

        trigger_gpio(21);

        // Test for HardFault
        // unsafe {
        //     core::ptr::write_volatile(0xFFFF_FFFC as *mut u32, 0x1234_5678);
        // }

        syscall::sleep_ms(2000);
    }
}
