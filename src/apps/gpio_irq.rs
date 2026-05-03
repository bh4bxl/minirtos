use crate::sys::{
    device_driver::{self, DeviceIrq, DeviceIrqEvent},
    sync::event::Event,
    syscall,
    task::{Priority, TaskStack},
};

const SHELL_PRIO: u8 = 100;

const SHELL_STACK_SIZE: usize = 1024;
static SHELL_STACK: TaskStack<SHELL_STACK_SIZE> = TaskStack::new();

pub fn start_gpio_irq_test() -> Result<(), &'static str> {
    if let Err(x) = syscall::thread_create(
        shell_task_entry,
        core::ptr::null_mut(),
        SHELL_STACK.get(),
        Priority(SHELL_PRIO),
        "gpio_irq",
    ) {
        Err(x)
    } else {
        Ok(())
    }
}

static GPIO27_EVENT: Event = Event::new(false);

/// Thread entry
extern "C" fn shell_task_entry(_arg: *mut ()) -> ! {
    let gpio = match device_driver::driver_manager().open_device(device_driver::DeviceType::Gpio, 0)
    {
        Some(dev) => dev,
        None => loop {
            defmt::warn!("No uart device found");
            cortex_m::asm::wfi();
        },
    };
    gpio.set_irq_callback(Some(gpio_irq_callback)).ok();

    loop {
        GPIO27_EVENT.wait();
        defmt::info!("GPIO27 triggered @{}", syscall::get_tick());

        let wlan = device_driver::driver_manager()
            .open_device(device_driver::DeviceType::Wlan, 0)
            .unwrap();

        let data = [0xAA, 0x55, 0xAA, 0x55, 0x12, 0x34, 0x56, 0x78];
        wlan.write(&data).unwrap();
    }
}

fn gpio_irq_callback(irq: DeviceIrq) {
    if irq.event != DeviceIrqEvent::Gpio {
        return;
    }

    if irq.data & 0xff == 27 {
        GPIO27_EVENT.signal();
    }
}
