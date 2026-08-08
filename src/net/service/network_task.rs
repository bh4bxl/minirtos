use crate::sys::{
    SysError,
    sync::message_queue::MessageQueue,
    syscall::sleep_ms,
    task::{Priority, Task},
};

use super::{
    super::{
        NetResponse, complete_request,
        core::{WifiAuth, WifiConnectFailure},
        request::NetCommand,
    },
    network_stack::NetworkStack,
    wlan_controller::{WlanController, WlanControllerEvent},
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
pub enum NetUtilityCommand {
    Resolve(FixedStr<128>),
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
pub enum NetUtilityResult {
    DhcpConfigured(Ipv4Config),
    DhcpDeconfigured,
    Dns(DnsEvent),
}

pub static WLAN_CMD_QUEUE: MessageQueue<WlanCommand, 4> = MessageQueue::new();
pub static WLAN_RESULT_QUEUE: MessageQueue<WlanResult, 8> = MessageQueue::new();
pub static NET_UTILITY_CMD_QUEUE: MessageQueue<NetUtilityCommand, 4> = MessageQueue::new();
pub static NET_UTILITY_RESULT_QUEUE: MessageQueue<NetUtilityResult, 8> = MessageQueue::new();

pub static NET_CMD_QUEUE: MessageQueue<NetCommand, 8> = MessageQueue::new();

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
    let mut wlan = WlanController::new();
    let mut stack = NetworkStack::new();

    if let Err(e) = wlan.wifi_on() {
        log_network_error("wifi_on", e);
        return;
    }

    loop {
        /*
         * Receive frames from the WLAN driver.
         */
        wlan.poll_rx();

        /*
         * Always poll the network stack.
         *
         * Even when the WLAN link is down, pending framework requests
         * still need to complete with NetworkDown.
         */
        stack.poll();

        if wlan.is_connected() {
            if let Err(e) = wlan.drain_tx() {
                log_network_error("drain_tx", e);
            }
        }

        wlan.poll_state();

        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            handle_wlan_command(&mut wlan, cmd);
        }

        if let Some(cmd) = NET_UTILITY_CMD_QUEUE.try_recv() {
            handle_net_utility_command(&mut stack, cmd);
        }

        /*
         * Process several framework requests per iteration.
         */
        for _ in 0..4 {
            let Some(command) = NET_CMD_QUEUE.try_recv() else {
                break;
            };

            handle_framework_command(&mut stack, command);
        }

        for _ in 0..8 {
            let Some(event) = wlan.take_event() else {
                break;
            };
            handle_wlan_event(&mut wlan, &mut stack, event);
        }

        for _ in 0..8 {
            let Some(event) = stack.take_event() else {
                break;
            };
            handle_net_event(event);
        }

        sleep_ms(10);
    }
}

fn handle_wlan_command(wlan: &mut WlanController, cmd: WlanCommand) {
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

fn handle_net_utility_command(net: &mut NetworkStack, cmd: NetUtilityCommand) {
    match cmd {
        NetUtilityCommand::Resolve(hostname) => {
            if let Err(e) = net.resolve(hostname.as_str()) {
                log_network_error("resolve", e);
                NET_UTILITY_RESULT_QUEUE.send(NetUtilityResult::Dns(DnsEvent::Failed));
            }
        }
    }
}

fn handle_framework_command(stack: &mut NetworkStack, command: NetCommand) {
    match command {
        NetCommand::IcmpEcho {
            request,
            target,
            timeout_ms,
        } => {
            if let Err(error) = stack.icmp_echo(request, target, timeout_ms) {
                complete_request(request, NetResponse::Error(error));
            }
        }
        NetCommand::TcpOpen { request } => match stack.tcp_open() {
            Ok(socket) => {
                complete_request(request, NetResponse::TcpOpened { socket });
            }

            Err(error) => {
                complete_request(request, NetResponse::Error(error));
            }
        },

        NetCommand::TcpConnect {
            request,
            socket,
            remote,
            timeout_ms,
        } => {
            if let Err(error) = stack.tcp_connect(request, socket, remote, timeout_ms) {
                complete_request(request, NetResponse::Error(error));
            }
        }

        NetCommand::TcpSend {
            request,
            socket,
            buffer,
            len,
            timeout_ms,
        } => {
            if let Err(error) = stack.tcp_send(request, socket, buffer, len, timeout_ms) {
                complete_request(request, NetResponse::Error(error));
            }
        }

        NetCommand::TcpRecv {
            request,
            socket,
            buffer,
            max_len,
            timeout_ms,
        } => {
            if let Err(error) = stack.tcp_recv(request, socket, buffer, max_len, timeout_ms) {
                complete_request(request, NetResponse::Error(error));
            }
        }

        NetCommand::TcpClose { request, socket } => {
            if let Err(error) = stack.tcp_close(request, socket) {
                complete_request(request, NetResponse::Error(error));
            }
        }

        NetCommand::TcpAbort { socket } => {
            stack.tcp_abort(socket);
        }
    }
}

fn handle_scan(wlan: &mut WlanController) {
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

fn handle_wlan_event(
    wlan: &mut WlanController,
    net: &mut NetworkStack,
    event: WlanControllerEvent,
) {
    match event {
        WlanControllerEvent::LinkConnected => {
            if let Some(mac) = wlan.mac_addr() {
                net.config(mac, 0x1234_5678);
            }
            WLAN_RESULT_QUEUE.send(WlanResult::LinkConnected);
        }

        WlanControllerEvent::LinkDisconnected => {
            net.reset();
            WLAN_RESULT_QUEUE.send(WlanResult::Disconnected);
        }

        WlanControllerEvent::ConnectFailed(failure) => {
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
            NET_UTILITY_RESULT_QUEUE.send(NetUtilityResult::DhcpConfigured(config));
        }
        NetEvent::DhcpDeconfigured => {
            NET_UTILITY_RESULT_QUEUE.send(NetUtilityResult::DhcpDeconfigured);
        }
        NetEvent::Dns(event) => NET_UTILITY_RESULT_QUEUE.send(NetUtilityResult::Dns(event)),
        NetEvent::IcmpReply {
            request,
            addr,
            sequence,
            bytes,
            rtt_ms,
        } => {
            complete_request(
                request,
                NetResponse::IcmpReply {
                    addr,
                    sequence,
                    bytes,
                    rtt_ms,
                },
            );
        }
        NetEvent::IcmpError { request, error } => {
            complete_request(request, NetResponse::Error(error));
        }
        NetEvent::TcpConnected { request } => {
            complete_request(request, NetResponse::TcpConnected);
        }
        NetEvent::TcpSent { request, len } => {
            complete_request(request, NetResponse::TcpSent { len });
        }
        NetEvent::TcpReceived { request, len } => {
            complete_request(request, NetResponse::TcpReceived { len });
        }
        NetEvent::TcpClosed { request } => {
            complete_request(request, NetResponse::TcpClosed);
        }
        NetEvent::TcpError { request, error } => {
            complete_request(request, NetResponse::Error(error));
        }
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
