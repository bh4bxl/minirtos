use core::net::Ipv4Addr;

use crate::net::request_api;

use super::NetResult;

const DEFAULT_PING_TIMEOUT_MS: u64 = 3000;

#[derive(Clone, Copy, Debug)]
pub struct PingReply {
    pub addr: Ipv4Addr,
    pub sequence: u16,
    pub bytes: usize,
    pub rtt_ms: u64,
}

pub fn ping(target: Ipv4Addr) -> NetResult<PingReply> {
    ping_timeout(target, DEFAULT_PING_TIMEOUT_MS)
}

pub fn ping_timeout(target: Ipv4Addr, timeout_ms: u64) -> NetResult<PingReply> {
    request_api::icmp_ping(target, timeout_ms)
}
