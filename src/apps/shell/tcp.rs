#![cfg(feature = "cyw43")]

use core::net::Ipv4Addr;

use super::{ShellApp, take_context};
use crate::{
    println,
    services::wlan_service::{FixedStr, TcpEvent},
    sys::task::Priority,
};

use super::super::wlan::{WLAN_CMD_QUEUE, WLAN_RESULT_QUEUE, WlanCmd, WlanResult};

const TCP_PRIO: u8 = 100;
const TCP_STACK_SIZE: usize = 512;
const DEFAULT_MESSAGE: &str = "miniRTOS";

extern "C" fn tcp_task(arg: *mut ()) {
    let context = unsafe { take_context(arg) };
    let mut argv = context.args();

    let Some(target_text) = argv.next() else {
        print_usage();
        return;
    };

    let target = match target_text.parse::<Ipv4Addr>() {
        Ok(target) => target,
        Err(_) => {
            println!("tcp: invalid IPv4 address: {}", target_text);
            return;
        }
    };

    let Some(port_text) = argv.next() else {
        print_usage();
        return;
    };

    let port = match port_text.parse::<u16>() {
        Ok(port) if port != 0 => port,
        _ => {
            println!("tcp: invalid port: {}", port_text);
            return;
        }
    };

    let message = argv.next().unwrap_or(DEFAULT_MESSAGE);

    if argv.next().is_some() {
        println!("tcp: message must not contain spaces");
        print_usage();
        return;
    }

    let Some(data) = FixedStr::<128>::from_str(message) else {
        println!("tcp: message is too long (maximum 128 bytes)");
        return;
    };

    println!("TCP echo {}:{}: sending {} bytes", target, port, data.len);

    WLAN_CMD_QUEUE.send(WlanCmd::TcpEcho { target, port, data });

    loop {
        match WLAN_RESULT_QUEUE.recv() {
            WlanResult::Tcp(event) => {
                print_result(event);
                return;
            }

            WlanResult::TcpFailed => {
                println!("tcp: failed to start echo test");
                return;
            }

            _ => {}
        }
    }
}

fn print_result(event: TcpEvent) {
    match event {
        TcpEvent::EchoReply {
            addr,
            port,
            data,
            elapsed_ms,
        } => {
            let payload = core::str::from_utf8(&data.buf[..data.len]).unwrap_or("<binary data>");

            println!(
                "reply from {}:{}: {} bytes, time={} ms: {}",
                addr, port, data.len, elapsed_ms, payload
            );
        }

        TcpEvent::ConnectFailed { addr, port } => {
            println!("tcp: failed to connect to {}:{}", addr, port);
        }

        TcpEvent::Timeout { addr, port } => {
            println!("tcp: echo {}:{} timed out", addr, port);
        }

        TcpEvent::Closed { addr, port } => {
            println!("tcp: connection to {}:{} closed", addr, port);
        }

        TcpEvent::NetworkDown { addr, port } => {
            println!("tcp: network down while connecting to {}:{}", addr, port);
        }
    }
}

fn print_usage() {
    println!("usage: tcp <ip-address> <port> [message]");
    println!("example: tcp 192.168.1.100 7000 hello");
}

pub(super) static TCP_APP: ShellApp = ShellApp::new(
    "tcp",
    "Run a TCP echo test",
    tcp_task,
    TCP_STACK_SIZE,
    Priority(TCP_PRIO),
);
