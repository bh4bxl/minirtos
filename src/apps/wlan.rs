#![cfg(feature = "cyw43")]
use core::{sync::atomic::AtomicBool, sync::atomic::Ordering};

use crate::{
    net::WifiAuth,
    services::wlan_service::{FixedStr, Ipv4Config, PingEvent, WlanService, WlanServiceEvent},
    sys::{
        SysError,
        device_driver::{self, DeviceIrq, DeviceIrqEvent},
        sync::message_queue::MessageQueue,
        syscall::sleep_ms,
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

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ConnectError {
    StartFailed,
    ConnectFailed,
    LinkLost,
    DhcpLost,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum WlanResult {
    ScanCompleted { count: usize },
    ScanFailed,

    ConnectStarted,
    LinkConnected,
    DhcpConfigured(Ipv4Config),
    ConnectFailed(ConnectError),

    Disconnected,
    DisconnectFailed,

    Ping(PingEvent),
}

pub static WLAN_CMD_QUEUE: MessageQueue<WlanCmd, 4> = MessageQueue::new();
pub static WLAN_RESULT_QUEUE: MessageQueue<WlanResult, 8> = MessageQueue::new();

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
extern "C" fn wlan_task_entry(_arg: *mut ()) {
    let gpio = match device_driver::driver_manager().open_device(device_driver::DeviceType::Gpio, 0)
    {
        Ok(dev) => dev,
        Err(e) => {
            defmt::warn!("Open uart device failed {}.", e as i32);
            return;
        }
    };
    gpio.set_irq_callback(Some(gpio_irq_callback)).ok();

    let mut wlan_srv = WlanService::new();

    wlan_srv.wifi_on();

    loop {
        wlan_srv.poll();

        // Shell command
        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            match cmd {
                WlanCmd::Scan => match wlan_srv.wifi_scan(20_000) {
                    Some(results) => {
                        results.iter().for_each(|r| {
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
                                    core::str::from_utf8(&r.ssid[..r.ssid_len as usize])
                                        .unwrap_or("<Invalid SSID>")
                                } else {
                                    "<Hidden SSID>"
                                },
                            );
                        });

                        WLAN_RESULT_QUEUE.send(WlanResult::ScanCompleted {
                            count: results.len(),
                        });
                    }

                    None => {
                        WLAN_RESULT_QUEUE.send(WlanResult::ScanFailed);
                    }
                },

                WlanCmd::Connect {
                    ssid,
                    password,
                    auth,
                } => {
                    WLAN_RESULT_QUEUE.send(WlanResult::ConnectStarted);

                    if !wlan_srv.wifi_connect(ssid, password, auth) {
                        WLAN_RESULT_QUEUE
                            .send(WlanResult::ConnectFailed(ConnectError::StartFailed));
                    }
                }
                WlanCmd::Disconnect => {
                    wlan_srv.wifi_disconnect();
                }
                WlanCmd::Ping => {
                    if !wlan_srv.ping_gateway() {
                        crate::println!("ping: failed to send");
                    }
                }
            }
        }

        if let Some(event) = wlan_srv.take_event() {
            match event {
                WlanServiceEvent::LinkConnected => {
                    WLAN_RESULT_QUEUE.send(WlanResult::LinkConnected);
                }

                WlanServiceEvent::LinkDisconnected => {
                    WLAN_RESULT_QUEUE.send(WlanResult::Disconnected);
                }

                WlanServiceEvent::ConnectFailed => {
                    wlan_srv.wifi_disconnect();
                    WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(ConnectError::ConnectFailed));
                }

                WlanServiceEvent::DhcpConfigured(config) => {
                    WLAN_RESULT_QUEUE.send(WlanResult::DhcpConfigured(config));
                }

                WlanServiceEvent::DhcpDeconfigured => {
                    WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(ConnectError::DhcpLost));
                }

                WlanServiceEvent::Ping(event) => {
                    WLAN_RESULT_QUEUE.send(WlanResult::Ping(event));
                }
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
