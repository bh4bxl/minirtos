#![cfg(feature = "cyw43")]
use core::net::Ipv4Addr;

use crate::apps::shell::ShellApp;
use crate::net::{NetError, ping_timeout, resolve};
use crate::println;
use crate::sys::task::Priority;

const PING_PRIO: u8 = 100;
const PING_STACK_SIZE: usize = 512;
const PING_COUNT: usize = 4;
const PING_TIMEOUT_MS: u64 = 3000;

extern "C" fn ping_task(arg: *mut ()) {
    let context = unsafe { super::take_context(arg) };

    let Some(target_text) = context.arg(0) else {
        println!("Usage: ping <ip-address|hostname>");
        return;
    };

    let target = match target_text.parse::<Ipv4Addr>() {
        Ok(ip) => ip,

        Err(_) => match resolve(target_text) {
            Ok(ip) => ip,

            Err(NetError::TimedOut) => {
                println!("ping: DNS lookup for {} timed out", target_text);
                return;
            }

            Err(NetError::NetworkDown) => {
                println!("ping: network is down");
                return;
            }

            Err(_) => {
                println!("ping: cannot resolve {}", target_text);
                return;
            }
        },
    };

    if target_text.parse::<Ipv4Addr>().is_ok() {
        println!("PING {}", target);
    } else {
        println!("PING {} ({})", target_text, target);
    }

    for _ in 0..PING_COUNT {
        match ping_timeout(target, PING_TIMEOUT_MS) {
            Ok(reply) => {
                println!(
                    "{} bytes from {}: icmp_seq={} time={} ms",
                    reply.bytes, reply.addr, reply.sequence, reply.rtt_ms,
                );
            }

            Err(NetError::TimedOut) => {
                println!("Request timeout for {}", target);
            }

            Err(NetError::NetworkDown) => {
                println!("ping: network is down, target={}", target);

                return;
            }

            Err(error) => {
                println!("ping: failed: {:?}", error);

                return;
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
