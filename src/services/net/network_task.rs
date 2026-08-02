use core::net::Ipv4Addr;

use crate::{
    net::{WifiAuth, WifiConnectFailure},
    services::net::net_service::NetService,
    sys::{
        SysError,
        sync::message_queue::MessageQueue,
        syscall::sleep_ms,
        task::{Priority, Task},
    },
};

use super::{
    wlan_service::{WlanService, WlanServiceEvent},
    *,
};

#[derive(Clone, Copy)]
pub enum WlanCommand {
    Scan,
    Connect {
        ssid: FixedStr<32>,
        password: Option<FixedStr<64>>,
        auth: WifiAuth,
    },
    Disconnect,
}

#[derive(Clone, Copy)]
pub enum NetCommand {
    Resolve(FixedStr<128>),
    Ping(Ipv4Addr),
    TcpEcho {
        target: Ipv4Addr,
        port: u16,
        data: FixedStr<128>,
    },
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ConnectError {
    StartFailed,
    ConnectFailed,
    AuthFailed,
    LinkLost,
}

#[derive(Clone, Copy, Debug)]
pub enum WlanResult {
    ScanCompleted { count: usize },
    ScanFailed,
    ConnectStarted,
    LinkConnected,
    ConnectFailed(ConnectError),
    Disconnected,
    DisconnectFailed,
}

#[derive(Clone, Copy, Debug)]
pub enum NetResult {
    DhcpConfigured(Ipv4Config),
    DhcpDeconfigured,
    Dns(DnsEvent),
    Ping(PingEvent),
    PingFailed,
    Tcp(TcpEvent),
    TcpFailed,
}

pub static WLAN_CMD_QUEUE: MessageQueue<WlanCommand, 4> = MessageQueue::new();
pub static WLAN_RESULT_QUEUE: MessageQueue<WlanResult, 8> = MessageQueue::new();
pub static NET_CMD_QUEUE: MessageQueue<NetCommand, 4> = MessageQueue::new();
pub static NET_RESULT_QUEUE: MessageQueue<NetResult, 8> = MessageQueue::new();

const NETWORK_PRIO: u8 = 150;
const NETWORK_STACK_SIZE: usize = 4096;

pub fn start_network() -> Result<(), SysError> {
    let mut network = Task::<NETWORK_STACK_SIZE>::new(network_task_entry)
        .priority(Priority(NETWORK_PRIO))
        .name("network");

    network.run()?;
    Ok(())
}

/// Network task entry. Owns both the WLAN link service and the IP stack.
extern "C" fn network_task_entry(_arg: *mut ()) {
    let mut wlan = WlanService::new();
    let mut net = NetService::new();

    if let Err(e) = wlan.wifi_on() {
        log_network_error("wifi_on", e);
        return;
    }

    loop {
        // Link RX -> smoltcp -> link TX.
        wlan.poll_rx();

        net.poll();

        if wlan.is_connected() {
            if let Err(e) = wlan.drain_tx() {
                log_network_error("drain_tx", e);
            }
        }

        wlan.poll_state();

        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            handle_wlan_command(&mut wlan, cmd);
        }

        if let Some(cmd) = NET_CMD_QUEUE.try_recv() {
            handle_net_command(&mut net, cmd);
        }

        for _ in 0..8 {
            let Some(event) = wlan.take_event() else {
                break;
            };
            handle_wlan_event(&mut wlan, &mut net, event);
        }

        for _ in 0..8 {
            let Some(event) = net.take_event() else {
                break;
            };
            handle_net_event(event);
        }

        sleep_ms(10);
    }
}

fn handle_wlan_command(wlan: &mut WlanService, cmd: WlanCommand) {
    match cmd {
        WlanCommand::Scan => handle_scan(wlan),

        WlanCommand::Connect {
            ssid,
            password,
            auth,
        } => match wlan.wifi_connect(ssid, password, auth) {
            Ok(()) => WLAN_RESULT_QUEUE.send(WlanResult::ConnectStarted),
            Err(e) => {
                log_network_error("wifi_connect", e);
                WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(ConnectError::StartFailed));
            }
        },

        WlanCommand::Disconnect => {
            if let Err(e) = wlan.wifi_disconnect() {
                log_network_error("wifi_disconnect", e);
                WLAN_RESULT_QUEUE.send(WlanResult::DisconnectFailed);
            }
        }
    }
}

fn handle_net_command(net: &mut NetService, cmd: NetCommand) {
    match cmd {
        NetCommand::Resolve(hostname) => {
            if let Err(e) = net.resolve(hostname.as_str()) {
                log_network_error("resolve", e);
                NET_RESULT_QUEUE.send(NetResult::Dns(DnsEvent::Failed));
            }
        }

        NetCommand::Ping(target) => {
            if let Err(e) = net.ping(target) {
                log_network_error("ping", e);
                NET_RESULT_QUEUE.send(NetResult::PingFailed);
            }
        }

        NetCommand::TcpEcho { target, port, data } => {
            if let Err(e) = net.tcp_echo(target, port, data) {
                log_network_error("tcp_echo", e);
                NET_RESULT_QUEUE.send(NetResult::TcpFailed);
            }
        }
    }
}

fn handle_scan(wlan: &mut WlanService) {
    match wlan.wifi_scan(20_000) {
        Ok(results) => {
            for result in &results {
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
            log_network_error("wifi_scan", e);
            WLAN_RESULT_QUEUE.send(WlanResult::ScanFailed);
        }
    }
}

fn handle_wlan_event(wlan: &mut WlanService, net: &mut NetService, event: WlanServiceEvent) {
    match event {
        WlanServiceEvent::LinkConnected => {
            if let Some(mac) = wlan.mac_addr() {
                net.config(mac, 0x1234_5678);
            }
            WLAN_RESULT_QUEUE.send(WlanResult::LinkConnected);
        }

        WlanServiceEvent::LinkDisconnected => {
            net.reset();
            WLAN_RESULT_QUEUE.send(WlanResult::Disconnected);
        }

        WlanServiceEvent::ConnectFailed(failure) => {
            net.reset();

            if let Err(e) = wlan.wifi_disconnect() {
                log_network_error("disconnect after connect failure", e);
            }

            let error = match failure {
                WifiConnectFailure::PskFailed {
                    status: 8,
                    reason: 14,
                } => ConnectError::AuthFailed,
                _ => ConnectError::ConnectFailed,
            };

            WLAN_RESULT_QUEUE.send(WlanResult::ConnectFailed(error));
        }
    }
}

fn handle_net_event(event: NetEvent) {
    match event {
        NetEvent::DhcpConfigured(config) => {
            NET_RESULT_QUEUE.send(NetResult::DhcpConfigured(config));
        }
        NetEvent::DhcpDeconfigured => {
            NET_RESULT_QUEUE.send(NetResult::DhcpDeconfigured);
        }
        NetEvent::Dns(event) => NET_RESULT_QUEUE.send(NetResult::Dns(event)),
        NetEvent::Ping(event) => NET_RESULT_QUEUE.send(NetResult::Ping(event)),
        NetEvent::Tcp(event) => NET_RESULT_QUEUE.send(NetResult::Tcp(event)),
    }
}

fn log_network_error(operation: &str, error: NetworkError) {
    match error {
        NetworkError::Driver(e) => {
            defmt::warn!("NETTASK: {} driver error: {}", operation, e as usize);
        }
        NetworkError::WifiOff => {
            defmt::warn!("NETTASK: {} failed: wifi is off", operation);
        }
        NetworkError::NetworkDown => {
            defmt::warn!("NETTASK: {} failed: network is down", operation);
        }
        NetworkError::Busy => {
            defmt::warn!("NETTASK: {} failed: busy", operation);
        }
        NetworkError::Timeout => {
            defmt::warn!("NETTASK: {} failed: timeout", operation);
        }
        NetworkError::NotReady => {
            defmt::warn!("NETTASK: {} failed: network not ready", operation);
        }
        _ => defmt::warn!("NETTASK: {} failed", operation),
    }
}
