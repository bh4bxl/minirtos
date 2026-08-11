#![cfg(feature = "cyw43")]

use core::net::{Ipv4Addr, SocketAddrV4};

use super::{ShellApp, take_context};
use crate::{
    net::{NetError, Read, TcpStream, Write},
    println,
    sys::{
        syscall,
        task::{Priority, Privilege},
    },
};

const TCP_PRIO: u8 = 100;
const TCP_STACK_SIZE: usize = 512;
const DEFAULT_MESSAGE: &str = "miniRTOS";
const MAX_MESSAGE_LEN: usize = 128;

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

    let payload = message.as_bytes();

    if payload.len() > MAX_MESSAGE_LEN {
        println!(
            "tcp: message is too long (maximum {} bytes)",
            MAX_MESSAGE_LEN
        );
        return;
    }

    let remote = SocketAddrV4::new(target, port);

    println!("TCP echo {}: sending {} bytes", remote, payload.len());

    let started_tick = syscall::get_tick();

    let mut stream = match TcpStream::connect(remote) {
        Ok(stream) => stream,

        Err(error) => {
            print_connect_error(remote, error);
            return;
        }
    };

    if let Err(error) = stream.write_all(payload) {
        print_io_error("send", remote, error);
        return;
    }

    let mut reply = [0u8; MAX_MESSAGE_LEN];
    let reply = &mut reply[..payload.len()];

    if let Err(error) = stream.read_exact(reply) {
        print_io_error("receive", remote, error);
        return;
    }

    let elapsed_ms = syscall::get_tick().wrapping_sub(started_tick);

    if reply != payload {
        let text = core::str::from_utf8(reply).unwrap_or("<binary data>");

        println!(
            "tcp: echo mismatch from {}: {} bytes: {}",
            remote,
            reply.len(),
            text
        );

        let _ = stream.close();
        return;
    }

    let text = core::str::from_utf8(reply).unwrap_or("<binary data>");

    println!(
        "reply from {}: {} bytes, time={} ms: {}",
        remote,
        reply.len(),
        elapsed_ms,
        text
    );

    if let Err(error) = stream.close() {
        println!("tcp: failed to close connection to {}: {:?}", remote, error);
    }
}

fn print_connect_error(remote: SocketAddrV4, error: NetError) {
    match error {
        NetError::NetworkDown => {
            println!("tcp: network is down while connecting to {}", remote);
        }

        NetError::ConnectionRefused => {
            println!("tcp: connection refused by {}", remote);
        }

        NetError::TimedOut => {
            println!("tcp: connection to {} timed out", remote);
        }

        NetError::NoSocketAvailable => {
            println!("tcp: no TCP socket available");
        }

        _ => {
            println!("tcp: failed to connect to {}: {:?}", remote, error);
        }
    }
}

fn print_io_error(operation: &str, remote: SocketAddrV4, error: NetError) {
    match error {
        NetError::NetworkDown => {
            println!(
                "tcp: network went down during {} with {}",
                operation, remote
            );
        }

        NetError::TimedOut => {
            println!("tcp: {} from {} timed out", operation, remote);
        }

        NetError::ConnectionReset => {
            println!(
                "tcp: connection to {} was reset during {}",
                remote, operation
            );
        }

        NetError::Closed => {
            println!("tcp: connection to {} closed during {}", remote, operation);
        }

        _ => {
            println!("tcp: {} failed for {}: {:?}", operation, remote, error);
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
    Privilege::Privileged,
);
