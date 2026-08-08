use core::net::Ipv4Addr;

use super::{NetResult, request_api};

const DEFAULT_DNS_TIMEOUT_MS: u64 = 5000;

pub fn resolve(hostname: &str) -> NetResult<Ipv4Addr> {
    resolve_timeout(hostname, DEFAULT_DNS_TIMEOUT_MS)
}

pub fn resolve_timeout(hostname: &str, timeout_ms: u64) -> NetResult<Ipv4Addr> {
    request_api::dns_resolve(hostname, timeout_ms)
}
