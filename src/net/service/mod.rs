use core::{fmt::Debug, net::Ipv4Addr};

use smoltcp::socket::dns::{QueryHandle, StartQueryError};

use crate::sys::device_driver::DevError;

use super::core::NetDevice;

pub mod network_stack;
pub mod network_task;
pub mod wlan_controller;

#[derive(Clone, Copy)]
pub struct FixedStr<const N: usize> {
    pub buf: [u8; N],
    pub len: usize,
}

impl<const N: usize> FixedStr<N> {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() > N {
            return None;
        }

        let mut out = Self {
            buf: [0; N],
            len: s.len(),
        };

        out.buf[..s.len()].copy_from_slice(s.as_bytes());
        Some(out)
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len]) }
    }
}

impl<const N: usize> Debug for FixedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ipv4Config {
    pub address: Ipv4Addr,
    pub prefix_len: u8,
    pub gateway: Option<Ipv4Addr>,
    pub dns: Option<Ipv4Addr>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum DnsEvent {
    Resolved { addr: Ipv4Addr },
    Failed,
    Timeout,
    NetworkDown,
}

#[derive(Clone, Copy)]
enum DnsState {
    Idle,
    Waiting {
        query: QueryHandle,
        started_tick: u64,
    },
    Done(DnsEvent),
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum PingEvent {
    Reply {
        addr: Ipv4Addr,
        seq: u16,
        len: usize,
        rtt_ms: u64,
    },
    Timeout {
        addr: Ipv4Addr,
        seq: u16,
    },
    SendFailed {
        addr: Ipv4Addr,
    },
    NetworkDown {
        addr: Ipv4Addr,
        seq: u16,
    },
}

#[derive(Clone, Copy, Debug)]
enum PingState {
    Idle,
    Waiting {
        target: Ipv4Addr,
        seq: u16,
        sent_tick: u64,
    },
    Done(PingEvent),
}

#[derive(Clone, Copy, Debug)]
pub enum TcpEvent {
    EchoReply {
        addr: Ipv4Addr,
        port: u16,
        data: FixedStr<128>,
        elapsed_ms: u64,
    },
    ConnectFailed {
        addr: Ipv4Addr,
        port: u16,
    },
    Timeout {
        addr: Ipv4Addr,
        port: u16,
    },
    Closed {
        addr: Ipv4Addr,
        port: u16,
    },
    NetworkDown {
        addr: Ipv4Addr,
        port: u16,
    },
}

#[derive(Clone, Copy)]
enum TcpState {
    Idle,
    Connecting {
        target: Ipv4Addr,
        port: u16,
        data: FixedStr<128>,
        started_tick: u64,
    },
    Sending {
        target: Ipv4Addr,
        port: u16,
        data: FixedStr<128>,
        sent: usize,
        started_tick: u64,
    },
    Receiving {
        target: Ipv4Addr,
        port: u16,
        expected: FixedStr<128>,
        received: FixedStr<128>,
        started_tick: u64,
    },
    Done(TcpEvent),
}

#[derive(Clone, Copy, Debug)]
pub enum NetEvent {
    DhcpConfigured(Ipv4Config),
    DhcpDeconfigured,
    Dns(DnsEvent),
    Ping(PingEvent),
    Tcp(TcpEvent),
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    WifiOff,
    NotReady,
    NetworkDown,
    Busy,
    Timeout,
    Driver(DevError),
    InvalidState,
    InvalidPacket,
    InvalidArgument,
    NoAddress,
    NoGateway,
    NoDnsServer,
    Dns(StartQueryError),
    IcmpBindFailed,
    IcmpSendFailed,
    QueueFull,
}

pub(super) static NETDEV: NetDevice = NetDevice::new();
