#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use crate::{
    bsp::board_init,
    sys::{
        arch::arm_cortex_m::start_first_task,
        device_driver::{self, DeviceType},
        sync::{event::Event, message_queue::MessageQueue, mutex::Mutex, semaphore::Semaphore},
        syscall::{self, sleep_ms},
        task::{Priority, TaskStack},
    },
};
use rp235x_hal as hal;

mod apps;
mod bsp;
mod drivers;
mod gui;
mod sys;

#[hal::entry]
fn main() -> ! {
    defmt::info!("MINI RTOS");

    match board_init() {
        Err(e) => defmt::error!("Error: {:?}", e as u16),
        Ok(()) => defmt::info!("Board {} initialized.", sys::board::board().board_name()),
    }

    m_info!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    sys::kernel_init().unwrap();

    apps::shell::start_shell().unwrap();

    apps::hmi::start_hmi().unwrap();

    syscall::thread_create(
        task1_entry,
        core::ptr::null_mut(),
        TASK1_STACK.get(),
        Priority(110),
        "task1",
    )
    .unwrap();

    syscall::thread_create(
        task2_entry,
        core::ptr::null_mut(),
        TASK2_STACK.get(),
        Priority(110),
        "task2",
    )
    .unwrap();

    unsafe {
        start_first_task();
    }
}

// Test tasks
const TEST_TASK_STACK_SIZE: usize = 128;
static TASK1_STACK: TaskStack<TEST_TASK_STACK_SIZE> = TaskStack::new();
static TASK2_STACK: TaskStack<TEST_TASK_STACK_SIZE> = TaskStack::new();

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

// Test for Event
static TEST_EVENT: Event = Event::new(false);

extern "C" fn task1_entry(_: *mut ()) -> ! {
    let mut cnt = 0u32;
    loop {
        cnt += 1;
        defmt::info!("task1 running {}", cnt);

        // For semaphore
        // defmt::info!("waiter: before wait");
        // TEST_SEM.wait();
        // defmt::info!("waiter: after wait");

        // For Mutex
        // M.lock();
        // print!("A ");
        // sleep_ms(500);
        // M.unlock();

        // For Event
        TEST_EVENT.signal();
        defmt::info!("task1 signals {}", cnt);
        sleep_ms(1000);

        trigger_gpio(19);

        // syscall::sleep_ms(1000);
    }
}

extern "C" fn task2_entry(_: *mut ()) -> ! {
    loop {
        // For semaphore
        // defmt::info!("signaler: signal");
        // TEST_SEM.signal();
        // sleep_ms(2000);

        // For Mutex
        // M.lock();
        // print!("B ");
        // sleep_ms(500);
        // M.unlock();

        // For Event
        defmt::info!("task2 block {}", syscall::get_tick());
        TEST_EVENT.wait();
        defmt::info!("task2 awake {}", syscall::get_tick());

        trigger_gpio(21);

        // Test for HardFault
        // unsafe {
        //     core::ptr::write_volatile(0xFFFF_FFFC as *mut u32, 0x1234_5678);
        // }

        // syscall::sleep_ms(2000);
    }
}
