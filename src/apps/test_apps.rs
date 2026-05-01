use crate::sys::{
    device_driver::{self, DeviceType},
    sync::event::Event,
    syscall,
    task::{Priority, TaskStack},
};

const TASK_PRIORTIY: u8 = 100;

const TEST_TASK_STACK_SIZE: usize = 128;
static TASK1_STACK: TaskStack<TEST_TASK_STACK_SIZE> = TaskStack::new();
static TASK2_STACK: TaskStack<TEST_TASK_STACK_SIZE> = TaskStack::new();

#[allow(dead_code)]
pub fn start_test_apps() -> Result<(), &'static str> {
    let Ok(_) = syscall::thread_create(
        task1_entry,
        core::ptr::null_mut(),
        TASK1_STACK.get(),
        Priority(TASK_PRIORTIY),
        "task1",
    ) else {
        return Err("Failed to create task1");
    };

    let Ok(_) = syscall::thread_create(
        task2_entry,
        core::ptr::null_mut(),
        TASK2_STACK.get(),
        Priority(TASK_PRIORTIY),
        "task2",
    ) else {
        return Err("Failed to create task2");
    };

    Ok(())
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
        syscall::sleep_ms(1000);

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
