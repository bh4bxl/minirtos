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
        Some("scan") => {
            if argv.next().is_some() {
                println!("usage: wifi scan");
                return;
            }

            println!("wifi scanning...");

            WLAN_CMD_QUEUE.send(WlanCmd::Scan);
            WLAN_SCAN_DONE.wait();

            println!("wifi scan done");
        }

        Some("connect") => {
            let ssid = match argv.next() {
                Some(v) => v,
                None => {
                    println!("missing ssid");
                    return;
                }
            };

            let password = argv.next();

            let (password, auth) = match password {
                Some(pw) => (Some(FixedStr::from_str(pw).unwrap()), WifiAuth::Wpa2AesPsk),

                None => (None, WifiAuth::Open),
            };

            println!("connecting to {}", ssid);

            WLAN_CMD_QUEUE.send(WlanCmd::Connect {
                ssid: FixedStr::from_str(ssid).unwrap(),
                password,
                auth,
            });

            WLAN_CONNECT_DONE.wait();

            println!("wifi connect done");
        }

        Some("disconnect") => {
            if argv.next().is_some() {
                println!("usage: wifi disconnect");
                return;
            }

            println!("disconnecting...");

            WLAN_CMD_QUEUE.send(WlanCmd::Disconnect);
            WLAN_DISCONNECT_DONE.wait();

            println!("wifi disconnect done");
        }

        Some("help") | None => {
            println!("wifi commands:");
            println!("  wifi scan");
            println!("  wifi connect <ssid> [password]");
            println!("  wifi disconnect");
        }

        Some(cmd) => {
            println!("unknown wifi command: {}", cmd);
        }
    }
}

pub(super) static WIFI_APP: ShellApp = ShellApp::new(
    "wifi",
    "Wi-Fi commands",
    wifi_task,
    WIFI_STACK_SIZE,
    Priority(WIFI_PRIO),
);
