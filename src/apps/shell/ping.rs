#![cfg(feature = "cyw43")]
use core::net::Ipv4Addr;

use crate::apps::shell::ShellApp;
use crate::net::service::{
    DnsEvent, FixedStr, PingEvent,
    network_task::{NET_CMD_QUEUE, NET_RESULT_QUEUE, NetCommand, NetResult},
};
use crate::println;
use crate::sys::task::Priority;

const PING_PRIO: u8 = 100;
const PING_STACK_SIZE: usize = 256;
const PING_COUNT: usize = 4;

extern "C" fn ping_task(arg: *mut ()) {
    let context = unsafe { super::take_context(arg) };

    let Some(target_text) = context.arg(0) else {
        println!("Usage: ping <ip-address>");
        return;
    };

    let target = if let Ok(ip) = target_text.parse::<Ipv4Addr>() {
        ip
    } else {
        // DNS
        let hostname = match FixedStr::<128>::from_str(target_text) {
            Some(host) => host,
            None => {
                println!("ping: hostname too long");
                return;
            }
        };

        NET_CMD_QUEUE.send(NetCommand::Resolve(hostname));

        loop {
            match NET_RESULT_QUEUE.recv() {
                NetResult::Dns(DnsEvent::Resolved { addr }) => break addr,

                NetResult::Dns(DnsEvent::Timeout) => {
                    println!("ping: cannot resolve {}", target_text);
                    return;
                }

                NetResult::Dns(DnsEvent::Failed) => {
                    println!("ping: cannot resolve {}", target_text);
                    return;
                }

                _ => {}
            }
        }
    };

    if target_text.parse::<Ipv4Addr>().is_ok() {
        println!("PING {}", target);
    } else {
        println!("PING {} ({})", target_text, target);
    }

    for _ in 0..PING_COUNT {
        NET_CMD_QUEUE.send(NetCommand::Ping(target));

        loop {
            match NET_RESULT_QUEUE.recv() {
                NetResult::Ping(PingEvent::Reply {
                    addr,
                    seq,
                    len,
                    rtt_ms,
                }) => {
                    println!(
                        "{} bytes from {}: icmp_seq={} time={} ms",
                        len, addr, seq, rtt_ms
                    );
                    break;
                }

                NetResult::Ping(PingEvent::Timeout { addr, seq }) => {
                    println!("Request timeout for {}, icmp_seq={}", addr, seq);
                    break;
                }

                NetResult::Ping(PingEvent::SendFailed { addr }) => {
                    println!("ping: failed to send to {}", addr);
                    return;
                }

                NetResult::Ping(PingEvent::NetworkDown { addr, seq }) => {
                    println!("ping: network is down, target={}, icmp_seq={}", addr, seq);
                    return;
                }

                NetResult::PingFailed => {
                    crate::println!("ping: failed to start");
                    return;
                }

                _ => {}
            }
        }

        crate::sys::syscall::sleep_ms(1000);
    }
}

pub(super) static PING_APP: ShellApp = ShellApp::new(
    "ping",
    "Ping an IPv4 address or hostname",
    ping_task,
    PING_STACK_SIZE,
    Priority(PING_PRIO),
);
