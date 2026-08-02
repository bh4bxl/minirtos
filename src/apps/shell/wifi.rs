#![cfg(feature = "cyw43")]

use crate::apps::shell::ShellApp;
use crate::net::WifiAuth;
use crate::println;
use crate::services::net::{
    FixedStr,
    network_task::{
        NET_RESULT_QUEUE, NetResult, WLAN_CMD_QUEUE, WLAN_RESULT_QUEUE, WlanCommand, WlanResult,
    },
};
use crate::sys::{syscall::sleep_ms, task::Priority};

const WIFI_PRIO: u8 = 100;
const WIFI_STACK_SIZE: usize = 512;

extern "C" fn wifi_task(arg: *mut ()) {
    let context = unsafe { super::take_context(arg) };
    let mut argv = context.args();

    match argv.next() {
        Some("scan") => wifi_scan(&mut argv),
        Some("connect") => wifi_connect(&mut argv),
        Some("disconnect") => wifi_disconnect(&mut argv),
        Some("help") | None => print_help(),
        Some(cmd) => {
            println!("unknown wifi command: {}", cmd);
            print_help();
        }
    }
}

fn wifi_scan<'a>(argv: &mut impl Iterator<Item = &'a str>) {
    if argv.next().is_some() {
        println!("usage: wifi scan");
        return;
    }

    println!("wifi scanning...");

    WLAN_CMD_QUEUE.send(WlanCommand::Scan);

    loop {
        match WLAN_RESULT_QUEUE.recv() {
            WlanResult::ScanCompleted { count } => {
                println!("wifi scan completed: {} network(s)", count);
                break;
            }

            WlanResult::ScanFailed => {
                println!("wifi scan failed");
                break;
            }

            // Ignore results unrelated to this command.
            _ => {}
        }
    }
}

fn wifi_connect<'a>(argv: &mut impl Iterator<Item = &'a str>) {
    let ssid = match argv.next() {
        Some(ssid) => ssid,
        None => {
            println!("usage: wifi connect <ssid> [password]");
            return;
        }
    };

    let password_arg = argv.next();

    if argv.next().is_some() {
        println!("usage: wifi connect <ssid> [password]");
        return;
    }

    let ssid = match FixedStr::<32>::from_str(ssid) {
        Some(ssid) => ssid,
        None => {
            println!("SSID is too long");
            return;
        }
    };

    let (password, auth) = match password_arg {
        Some(password) => {
            let password = match FixedStr::<64>::from_str(password) {
                Some(password) => password,
                None => {
                    println!("password is too long");
                    return;
                }
            };

            (Some(password), WifiAuth::Wpa2AesPsk)
        }

        None => (None, WifiAuth::Open),
    };

    WLAN_CMD_QUEUE.send(WlanCommand::Connect {
        ssid,
        password,
        auth,
    });

    let mut link_connected = false;

    loop {
        if let Some(result) = WLAN_RESULT_QUEUE.try_recv() {
            match result {
                WlanResult::ConnectStarted => {
                    println!("connecting...");
                }

                WlanResult::LinkConnected => {
                    link_connected = true;
                    println!("wifi connected");
                    println!("waiting for DHCP...");
                }

                WlanResult::ConnectFailed(reason) => {
                    println!("wifi connection failed: {:?}", reason);
                    return;
                }

                WlanResult::Disconnected => {
                    println!("wifi disconnected while connecting");
                    return;
                }

                _ => {}
            }
        }

        if link_connected {
            if let Some(result) = NET_RESULT_QUEUE.try_recv() {
                match result {
                    NetResult::DhcpConfigured(config) => {
                        println!("IP address: {}/{}", config.address, config.prefix_len);

                        if let Some(gateway) = config.gateway {
                            println!("Gateway:    {}", gateway);
                        }

                        if let Some(dns) = config.dns {
                            println!("DNS:        {}", dns);
                        }

                        return;
                    }

                    NetResult::DhcpDeconfigured => {
                        println!("DHCP configuration lost");
                        return;
                    }

                    _ => {}
                }
            }
        }

        sleep_ms(10);
    }
}

fn wifi_disconnect<'a>(argv: &mut impl Iterator<Item = &'a str>) {
    if argv.next().is_some() {
        println!("usage: wifi disconnect");
        return;
    }

    println!("disconnecting...");

    WLAN_CMD_QUEUE.send(WlanCommand::Disconnect);

    loop {
        match WLAN_RESULT_QUEUE.recv() {
            WlanResult::Disconnected => {
                println!("wifi disconnected");
                break;
            }

            WlanResult::DisconnectFailed => {
                println!("wifi disconnect failed");
                break;
            }

            _ => {}
        }
    }
}

fn print_help() {
    println!("wifi commands:");
    println!("  wifi scan");
    println!("  wifi connect <ssid> [password]");
    println!("  wifi disconnect");
}

pub(super) static WIFI_APP: ShellApp = ShellApp::new(
    "wifi",
    "Wi-Fi commands",
    wifi_task,
    WIFI_STACK_SIZE,
    Priority(WIFI_PRIO),
);
