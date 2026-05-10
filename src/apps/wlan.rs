use core::{sync::atomic::AtomicBool, sync::atomic::Ordering};

use crate::{
    drivers::wlan::cyw43::cyw43_country::*,
    net,
    sys::{
        device_driver::{self, DeviceIrq, DeviceIrqEvent},
        sync::{event::Event, message_queue::MessageQueue},
        syscall::{self, sleep_ms},
        task::{Priority, TaskStack},
    },
};

#[derive(Clone, Copy)]
pub enum WlanCmd {
    Scan,
}

pub static WLAN_CMD_QUEUE: MessageQueue<WlanCmd, 4> = MessageQueue::new();
pub static WLAN_SCAN_DONE: Event = Event::new(false);

const WLAN_PRIO: u8 = 150;

const WLAN_SIZE: usize = 4096;
static WLAN_STACK: TaskStack<WLAN_SIZE> = TaskStack::new();

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

    Ok(())
}

static GPIO15_PENDING: AtomicBool = AtomicBool::new(false);

/// Thread entry
extern "C" fn wlan_task_entry(_arg: *mut ()) -> ! {
    net::wlan().wifi_on(CYW43_COUNTRY_CANADA, None).unwrap();

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
        let _ = net::wlan().poll();

        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            match cmd {
                WlanCmd::Scan => {
                    defmt::info!("wifi scan requested");

                    if net::wlan().wifi_scan().is_ok() {
                        loop {
                            let _ = net::wlan().poll();

                            if net::wlan().wifi_scan_done().unwrap() {
                                break;
                            }

                            sleep_ms(10);
                        }

                        let mut res = heapless::Vec::new();
                        net::wlan().wifi_scan_results(&mut res).unwrap();

                        res.iter().for_each(|r| {
                            crate::println!(
                                "[{:>3} dBm] ch={:<3} {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}  {}",
                                r.rssi,
                                r.channel,
                                r.bssid[0],
                                r.bssid[1],
                                r.bssid[2],
                                r.bssid[3],
                                r.bssid[4],
                                r.bssid[5],
                                if r.ssid_len > 0 {
                                    core::str::from_utf8(&r.ssid).ok().unwrap()
                                } else {
                                    "<Hidden SSID>"
                                },
                            );
                        });
                        crate::println!("Total: {}", res.len());
                    } else {
                        defmt::warn!("wifi scan start failed");
                    }

                    WLAN_SCAN_DONE.signal();
                }
            }
        }

        if GPIO15_PENDING.swap(false, Ordering::AcqRel) {
            defmt::info!("GPIO15 triggered @{}", syscall::get_tick());

            if net::wlan().wifi_gpio_ctrl(0, level).is_ok() {
                level = !level;
            }
        }

        sleep_ms(10);
    }
}

fn gpio_irq_callback(irq: DeviceIrq) {
    if irq.event != DeviceIrqEvent::Gpio {
        return;
    }

    if irq.data & 0xff == 15 && irq.data & 0xff00 == 0 {
        GPIO15_PENDING.store(true, Ordering::Release);
    }
}
