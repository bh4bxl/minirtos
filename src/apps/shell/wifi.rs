#![cfg(feature = "cyw43")]
use super::super::wlan::*;
use crate::apps::shell::ShellApp;
use crate::net::WifiAuth;
use crate::println;
use crate::services::wlan_service::FixedStr;
use crate::sys::task::Priority;

const WIFI_PRIO: u8 = 100;
const WIFI_STACK_SIZE: usize = 256;

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

    WLAN_CMD_QUEUE.send(WlanCmd::Scan);

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

    WLAN_CMD_QUEUE.send(WlanCmd::Connect {
        ssid,
        password,
        auth,
    });

    loop {
        match WLAN_RESULT_QUEUE.recv() {
            WlanResult::ConnectStarted => {
                println!("connecting...");
            }

            WlanResult::LinkConnected => {
                println!("wifi connected");
                println!("waiting for DHCP...");
            }

            WlanResult::DhcpConfigured(config) => {
                println!("IP address: {}/{}", config.address, config.prefix_len);

                if let Some(gateway) = config.gateway {
                    println!("Gateway:    {}", gateway);
                }

                if let Some(dns) = config.dns {
                    println!("DNS:        {}", dns);
                }

                break;
            }

            WlanResult::ConnectFailed(reason) => {
                println!("wifi connection failed: {:?}", reason);
                break;
            }

            WlanResult::Disconnected => {
                println!("wifi disconnected while connecting");
                break;
            }

            _ => {}
        }
    }
}

fn wifi_disconnect<'a>(argv: &mut impl Iterator<Item = &'a str>) {
    if argv.next().is_some() {
        println!("usage: wifi disconnect");
        return;
    }

    println!("disconnecting...");

    WLAN_CMD_QUEUE.send(WlanCmd::Disconnect);

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
