#![cfg(feature = "cyw43")]
use super::super::wlan::*;
use crate::apps::shell::ShellApp;
use crate::println;
use crate::services::wlan_service::PingEvent;
use crate::sys::task::Priority;

const PING_PRIO: u8 = 100;
const PING_STACK_SIZE: usize = 256;

extern "C" fn ping_task(_arg: *mut ()) {
    println!("PING gateway");

    WLAN_CMD_QUEUE.send(WlanCmd::Ping);

    loop {
        match WLAN_RESULT_QUEUE.recv() {
            WlanResult::Ping(PingEvent::Reply { seq, len, rtt_ms }) => {
                println!(
                    "{} bytes from gateway: icmp_seq={} time={} ms",
                    len, seq, rtt_ms
                );
                break;
            }

            WlanResult::Ping(PingEvent::Timeout { seq }) => {
                println!("Request timeout for icmp_seq {}", seq);
                break;
            }

            _ => {}
        }
    }
}

pub(super) static PING_APP: ShellApp = ShellApp::new(
    "ping",
    "Ping gateway",
    ping_task,
    PING_STACK_SIZE,
    Priority(PING_PRIO),
);
