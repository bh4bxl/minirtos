#![cfg(feature = "cyw43")]
use core::{
    net::Ipv4Addr,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    net::WifiAuth,
    services::wlan_service::{
        DnsEvent, FixedStr, Ipv4Config, PingEvent, WlanService, WlanServiceError, WlanServiceEvent,
    },
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

    Resolve(FixedStr<128>),

    Ping(Ipv4Addr),
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

    Dns(DnsEvent),

    Ping(PingEvent),
    PingFailed,
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

/// WLAN service task entry.
extern "C" fn wlan_task_entry(_arg: *mut ()) {
    let gpio = match device_driver::driver_manager().open_device(device_driver::DeviceType::Gpio, 0)
    {
        Ok(dev) => dev,

        Err(e) => {
            defmt::warn!("WLANTASK: open GPIO device failed: {}", e as i32);
            return;
        }
    };

    if gpio.set_irq_callback(Some(gpio_irq_callback)).is_err() {
        defmt::warn!("WLANTASK: set GPIO IRQ callback failed");
        return;
    }

    let mut wlan_srv = WlanService::new();

    if let Err(e) = wlan_srv.wifi_on() {
        log_service_error("wifi_on", e);
        return;
    }

    loop {
        wlan_srv.poll();

        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            handle_command(&mut wlan_srv, cmd);
        }

        /*
         * Drain more than one service event per iteration.
         *
         * A disconnect may generate several events together:
         * - LinkDisconnected
         * - DNS failure/network-down
         * - Ping network-down
         * - DHCP deconfigured
         *
         * Only reading one event every 10 ms is functional, but draining
         * several events reduces latency and queue overflow risk.
         */
        for _ in 0..8 {
            let Some(event) = wlan_srv.take_event() else {
                break;
            };

            handle_service_event(&mut wlan_srv, event);
        }

        sleep_ms(10);
    }
}

fn handle_command(wlan_srv: &mut WlanService, cmd: WlanCmd) {
    match cmd {
        WlanCmd::Scan => {
            handle_scan(wlan_srv);
        }

        WlanCmd::Connect {
            ssid,
            password,
            auth,
        } => match wlan_srv.wifi_connect(ssid, password, auth) {
            Ok(()) => {
                WLAN_RESULT_QUEUE.send(WlanResult::ConnectStarted);
            }

            Err(e) => {
                log_service_error("wifi_connect", e);

                WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(ConnectError::StartFailed));
            }
        },

        WlanCmd::Disconnect => {
            match wlan_srv.wifi_disconnect() {
                Ok(()) => {
                    /*
                     * LinkDisconnected may also arrive asynchronously.
                     * Whether to send Disconnected here or only from the
                     * service event is a protocol design choice.
                     *
                     * This version waits for LinkDisconnected, avoiding
                     * duplicate Disconnected results.
                     */
                }

                Err(e) => {
                    log_service_error("wifi_disconnect", e);

                    WLAN_RESULT_QUEUE.send(WlanResult::DisconnectFailed);
                }
            }
        }

        WlanCmd::Resolve(hostname) => {
            if let Err(e) = wlan_srv.resolve(hostname.as_str()) {
                log_service_error("resolve", e);

                WLAN_RESULT_QUEUE.send(WlanResult::Dns(DnsEvent::Failed));
            }
        }

        WlanCmd::Ping(target) => {
            if let Err(e) = wlan_srv.ping(target) {
                log_service_error("ping", e);

                WLAN_RESULT_QUEUE.send(WlanResult::PingFailed);
            }
        }
    }
}

fn handle_scan(wlan_srv: &mut WlanService) {
    match wlan_srv.wifi_scan(20_000) {
        Ok(results) => {
            for result in results.iter() {
                let ssid = if result.ssid_len > 0 {
                    core::str::from_utf8(&result.ssid[..result.ssid_len as usize])
                        .unwrap_or("<Invalid SSID>")
                } else {
                    "<Hidden SSID>"
                };

                crate::println!(
                    "[{:>3} dBm] ch={:<3} \
                     {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}  {}",
                    result.rssi,
                    result.channel,
                    result.bssid[0],
                    result.bssid[1],
                    result.bssid[2],
                    result.bssid[3],
                    result.bssid[4],
                    result.bssid[5],
                    ssid,
                );
            }

            WLAN_RESULT_QUEUE.send(WlanResult::ScanCompleted {
                count: results.len(),
            });
        }

        Err(e) => {
            log_service_error("wifi_scan", e);

            WLAN_RESULT_QUEUE.send(WlanResult::ScanFailed);
        }
    }
}

fn handle_service_event(wlan_srv: &mut WlanService, event: WlanServiceEvent) {
    match event {
        WlanServiceEvent::LinkConnected => {
            WLAN_RESULT_QUEUE.send(WlanResult::LinkConnected);
        }

        WlanServiceEvent::LinkDisconnected => {
            WLAN_RESULT_QUEUE.send(WlanResult::Disconnected);
        }

        WlanServiceEvent::ConnectFailed => {
            /*
             * The service may already have cleared its local network state.
             * Calling driver disconnect here is still useful to reset a
             * failed connection attempt, but don't discard its error.
             */
            if let Err(e) = wlan_srv.wifi_disconnect() {
                log_service_error("disconnect after connect failure", e);
            }

            WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(ConnectError::ConnectFailed));
        }

        WlanServiceEvent::DhcpConfigured(config) => {
            WLAN_RESULT_QUEUE.send(WlanResult::DhcpConfigured(config));
        }

        WlanServiceEvent::DhcpDeconfigured => {
            WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(ConnectError::DhcpLost));
        }

        WlanServiceEvent::Dns(event) => {
            WLAN_RESULT_QUEUE.send(WlanResult::Dns(event));
        }

        WlanServiceEvent::Ping(event) => {
            WLAN_RESULT_QUEUE.send(WlanResult::Ping(event));
        }
    }
}

fn log_service_error(operation: &str, error: WlanServiceError) {
    match error {
        WlanServiceError::Driver(e) => {
            defmt::warn!("WLANTASK: {} driver error: {}", operation, e as usize);
        }

        WlanServiceError::WifiOff => {
            defmt::warn!("WLANTASK: {} failed: wifi is off", operation);
        }

        WlanServiceError::Busy => {
            defmt::warn!("WLANTASK: {} failed: busy", operation);
        }

        WlanServiceError::Timeout => {
            defmt::warn!("WLANTASK: {} failed: timeout", operation);
        }

        WlanServiceError::NotReady => {
            defmt::warn!("WLANTASK: {} failed: network not ready", operation);
        }

        _ => {
            defmt::warn!("WLANTASK: {} failed", operation);
        }
    }
}

fn gpio_irq_callback(irq: DeviceIrq) {
    if irq.event != DeviceIrqEvent::Gpio {
        return;
    }

    let pin = irq.data & 0xff;
    let level = (irq.data >> 8) & 0xff;

    if pin == 15 && level == 0 {
        GPIO15_PENDING.store(true, Ordering::Release);
    }
}
