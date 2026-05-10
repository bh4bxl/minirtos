use crate::{
    drivers::wlan::cyw43::cyw43_country::CYW43_COUNTRY_CANADA,
    net,
    sys::{
        device_driver::{self, DeviceIrq, DeviceIrqEvent},
        sync::event::Event,
        syscall::{self, sleep_ms},
        task::{Priority, TaskStack},
    },
};

const WLAN_PRIO: u8 = 150;

const WLAN_SIZE: usize = 4096;
static WLAN_STACK: TaskStack<WLAN_SIZE> = TaskStack::new();

const LED_SZIE: usize = 256;
static LED_STACK: TaskStack<LED_SZIE> = TaskStack::new();

pub fn start_wlan() -> Result<(), &'static str> {
    if let Err(x) = syscall::thread_create(
        wlan_task_entry,
        core::ptr::null_mut(),
        WLAN_STACK.get(),
        Priority(WLAN_PRIO),
        "gpio_irq",
    ) {
        return Err(x);
    }

    if let Err(x) = syscall::thread_create(
        led_task_entry,
        core::ptr::null_mut(),
        LED_STACK.get(),
        Priority(WLAN_PRIO),
        "led",
    ) {
        return Err(x);
    }

    Ok(())
}

static GPIO15_EVENT: Event = Event::new(false);

/// Thread entry
extern "C" fn wlan_task_entry(_arg: *mut ()) -> ! {
    net::wlan().wifi_on(CYW43_COUNTRY_CANADA, None).unwrap();
    net::wlan().wifi_scan().unwrap();

    loop {
        let _ = net::wlan().poll();

        sleep_ms(10);
    }
}

extern "C" fn led_task_entry(_arg: *mut ()) -> ! {
    let gpio = match device_driver::driver_manager().open_device(device_driver::DeviceType::Gpio, 0)
    {
        Some(dev) => dev,
        None => loop {
            defmt::warn!("No uart device found");
            cortex_m::asm::wfi();
        },
    };
    gpio.set_irq_callback(Some(gpio_irq_callback)).ok();
    let mut level = true;
    loop {
        GPIO15_EVENT.wait();
        defmt::info!("GPIO15 triggered @{}", syscall::get_tick());

        net::wlan().wifi_gpio_ctrl(0, level).unwrap();
        level = !level;
    }
}

fn gpio_irq_callback(irq: DeviceIrq) {
    if irq.event != DeviceIrqEvent::Gpio {
        return;
    }

    if irq.data & 0xff == 15 && irq.data & 0xff00 == 0 {
        GPIO15_EVENT.signal();
    }
}
