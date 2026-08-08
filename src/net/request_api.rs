use core::net::{Ipv4Addr, SocketAddrV4};

use super::{
    NetBuffer, NetError, NetResult, PingReply, SocketId,
    request::{NetCommand, NetResponse, RequestManager},
    service::FixedStr,
};

pub(super) fn dns_resolve(hostname: &str, timeout_ms: u64) -> NetResult<Ipv4Addr> {
    let hostname = FixedStr::<128>::from_str(hostname).ok_or(NetError::InvalidArgument)?;

    match RequestManager::submit(|request| NetCommand::DnsResolve {
        request,
        hostname,
        timeout_ms,
    })? {
        NetResponse::DnsResolved { addr } => Ok(addr),
        NetResponse::Error(error) => Err(error),
        _ => Err(NetError::Internal),
    }
}

pub(super) fn icmp_ping(target: Ipv4Addr, timeout_ms: u64) -> NetResult<PingReply> {
    match RequestManager::submit(|request| NetCommand::IcmpEcho {
        request,
        target,
        timeout_ms,
    })? {
        NetResponse::IcmpReply {
            addr,
            sequence,
            bytes,
            rtt_ms,
        } => Ok(PingReply {
            addr,
            sequence,
            bytes,
            rtt_ms,
        }),
        NetResponse::Error(error) => Err(error),
        _ => Err(NetError::Internal),
    }
}

pub(super) fn tcp_open() -> NetResult<SocketId> {
    match RequestManager::submit(|request| NetCommand::TcpOpen { request })? {
        NetResponse::TcpOpened { socket } => Ok(socket),
        NetResponse::Error(error) => Err(error),
        _ => Err(NetError::Internal),
    }
}

pub(super) fn tcp_connect(
    socket: SocketId,
    remote: SocketAddrV4,
    timeout_ms: u64,
) -> NetResult<()> {
    match RequestManager::submit(|request| NetCommand::TcpConnect {
        request,
        socket,
        remote,
        timeout_ms,
    })? {
        NetResponse::TcpConnected => Ok(()),
        NetResponse::Error(error) => Err(error),
        _ => Err(NetError::Internal),
    }
}

pub(super) fn tcp_send(socket: SocketId, buf: &[u8], timeout_ms: u64) -> NetResult<usize> {
    if buf.is_empty() {
        return Ok(0);
    }

    let mut buffer = NetBuffer::allocate()?;
    let count = buffer.write(buf)?;

    let response = RequestManager::submit(|request| NetCommand::TcpSend {
        request,
        socket,
        buffer: buffer.id(),
        len: count,
        timeout_ms,
    })?;

    /*
     * The request is complete, so NetworkTask no longer uses this buffer.
     * It is returned to the pool when `buffer` drops.
     */
    match response {
        NetResponse::TcpSent { len } => Ok(len),
        NetResponse::Error(error) => Err(error),
        _ => Err(NetError::Internal),
    }
}

pub(super) fn tcp_recv(socket: SocketId, buf: &mut [u8], timeout_ms: u64) -> NetResult<usize> {
    if buf.is_empty() {
        return Ok(0);
    }

    let buffer = NetBuffer::allocate()?;
    let max_len = buf.len().min(buffer.capacity());

    let response = RequestManager::submit(|request| NetCommand::TcpRecv {
        request,
        socket,
        buffer: buffer.id(),
        max_len,
        timeout_ms,
    })?;

    match response {
        NetResponse::TcpReceived { len } => {
            if len > max_len {
                return Err(NetError::Internal);
            }

            let copied = buffer.read(&mut buf[..len])?;

            if copied != len {
                return Err(NetError::Internal);
            }

            Ok(copied)
        }

        NetResponse::Error(error) => Err(error),

        _ => Err(NetError::Internal),
    }
}

pub(super) fn tcp_close(socket: SocketId) -> NetResult<()> {
    match RequestManager::submit(|request| NetCommand::TcpClose { request, socket })? {
        NetResponse::TcpClosed => Ok(()),
        NetResponse::Error(error) => Err(error),
        _ => Err(NetError::Internal),
    }
}

pub(super) fn tcp_abort(socket: SocketId) -> NetResult<()> {
    RequestManager::send(NetCommand::TcpAbort { socket });
    Ok(())
}
