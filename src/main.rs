#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use crate::{
    apps::shell::shell_task_entry,
    bsp::board_init,
    sys::{
        arch::arm_cortex_m::start_first_task,
        device_driver::{self, DeviceType},
        sync::{message_queue::MessageQueue, mutex::Mutex, semaphore::Semaphore},
        syscall::{self, sleep_ms},
        task::Priority,
    },
};
use rp235x_hal as hal;

mod apps;
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

    sys::kernel_init().unwrap();

    syscall::thread_create(
        shell_task_entry,
        core::ptr::null_mut(),
        Priority(100),
        "shell",
    )
    .unwrap();

    syscall::thread_create(task1_entry, core::ptr::null_mut(), Priority(110), "task1").unwrap();

    syscall::thread_create(task2_entry, core::ptr::null_mut(), Priority(110), "task2").unwrap();

    unsafe {
        start_first_task();
    }
}

// Test tasks

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

// Test for Semaphore
// static TEST_SEM: Semaphore = Semaphore::new(0);

// Test for Mutex
// static M: Mutex = Mutex::new();

// Test for MessageQueue
static Q: MessageQueue<u32, 4> = MessageQueue::new();

extern "C" fn task1_entry(_: *mut ()) -> ! {
    let mut cnt = 0u32;
    loop {
        cnt += 1;
        defmt::info!("task1 running {}", cnt);
        // m_info!("task1 running {}", cnt);

        // For semaphore
        // m_info!("waiter: before wait");
        // TEST_SEM.wait();
        // m_info!("waiter: after wait");

        // For Mutex
        // M.lock();
        // print!("A ");
        // sleep_ms(500);
        // M.unlock();

        // For MessageQueue
        Q.send(cnt);
        m_info!("task1 send {}", cnt);
        sleep_ms(1000);

        trigger_gpio(19);

        // syscall::sleep_ms(1000);
    }
}

extern "C" fn task2_entry(_: *mut ()) -> ! {
    let mut cnt = u32::MAX;
    loop {
        cnt -= 1;
        defmt::info!("task2 running {}", cnt);
        // m_info!("task2 running {}", cnt);

        // syscall::sleep_ms(1000);

        // For semaphore
        // m_info!("signaler: signal");
        // TEST_SEM.signal();

        // For Mutex
        // M.lock();
        // print!("B ");
        // sleep_ms(500);
        // M.unlock();

        // For MessageQueue
        let v = Q.recv();
        m_info!("task2 recv {}", v);

        trigger_gpio(21);

        let spi = device_driver::driver_manager().open_device(DeviceType::Spi, 0);
        if let Some(spi) = spi {
            let data = [0xAA; 512];
            defmt::info!("spi write");
            spi.write(&data).unwrap();
        }

        // Test for HardFault
        // unsafe {
        //     core::ptr::write_volatile(0xFFFF_FFFC as *mut u32, 0x1234_5678);
        // }

        // syscall::sleep_ms(2000);
    }
}
