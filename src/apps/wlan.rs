#![allow(dead_code)]
use core::{sync::atomic::AtomicBool, sync::atomic::Ordering};

use crate::{
    net::{self, WifiAuth, WifiState},
    services::wlan_service::{FixedStr, PingEvent, WlanService},
    sys::{
        SysError,
        device_driver::{self, DeviceIrq, DeviceIrqEvent},
        sync::{event::Event, message_queue::MessageQueue},
        syscall::{self, sleep_ms},
        task::{Priority, Task},
    },
};

#[derive(Clone, Copy)]
pub enum WlanCmd {
    Scan,
    Connect {
        ssid: FixedStr<32>,
        password: Option<FixedStr<64>>,
        auth: WifiAuth,
    },
    Disconnect,
    Ping,
}

pub static WLAN_CMD_QUEUE: MessageQueue<WlanCmd, 4> = MessageQueue::new();
pub static WLAN_SCAN_DONE: Event = Event::new(false);
pub static WLAN_CONNECT_DONE: Event = Event::new(false);
pub static WLAN_DISCONNECT_DONE: Event = Event::new(false);

const WLAN_PRIO: u8 = 150;

const WLAN_SIZE: usize = 4096;

pub fn start_wlan() -> Result<(), SysError> {
    let mut wlan = Task::<WLAN_SIZE>::new(wlan_task_entry)
        .priority(Priority(WLAN_PRIO))
        .name("wlan");
    wlan.run()?;

    Ok(())
}

static GPIO15_PENDING: AtomicBool = AtomicBool::new(false);

/// Thread entry
extern "C" fn wlan_task_entry(_arg: *mut ()) -> ! {
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

    let mut wlan_srv = WlanService::new();

    wlan_srv.wifi_on();

    loop {
        wlan_srv.poll_wifi();

        // Shell command
        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            match cmd {
                WlanCmd::Scan => {
                    if let Some(res) = wlan_srv.wifi_scan(20_000) {
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
                WlanCmd::Connect {
                    ssid,
                    password,
                    auth,
                } => {
                    wlan_srv.wifi_connect(ssid, password, auth);
                }
                WlanCmd::Disconnect => {
                    defmt::info!("wifi disconnect requested");
                    wlan_srv.wifi_disconnect();
                }
                WlanCmd::Ping => {
                    crate::println!("PING gateway");
                    if !wlan_srv.ping_gateway() {
                        crate::println!("ping: failed to send");
                    }
                }
            }
        }

        wlan_srv.poll_smoltcp();

        if let Some(event) = wlan_srv.take_ping_event() {
            match event {
                PingEvent::Reply { seq, len, rtt_ms } => {
                    crate::println!(
                        "{} bytes from gateway: icmp_seq={} time={} ms",
                        len,
                        seq,
                        rtt_ms,
                    );
                }

                PingEvent::Timeout { seq } => {
                    crate::println!("Request timeout for icmp_seq {}", seq,);
                }
            }
        }

        if let Some((_old, new)) = wlan_srv.poll_wifi_state_change() {
            if new == WifiState::Connected {
                WLAN_CONNECT_DONE.signal();
            }

            if new == WifiState::Down {
                WLAN_DISCONNECT_DONE.signal();
            }
        }

        // Key
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
