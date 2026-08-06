use core::net::SocketAddrV4;

use super::{NetError, NetResult, SocketId, request_api};

const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 5000;
const DEFAULT_IO_TIMEOUT_MS: u64 = 5000;

pub struct TcpStream {
    socket: SocketId,
    closed: bool,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
}

impl TcpStream {
    pub fn connect(remote: SocketAddrV4) -> NetResult<Self> {
        Self::connect_timeout(remote, DEFAULT_CONNECT_TIMEOUT_MS)
    }

    pub fn connect_timeout(remote: SocketAddrV4, timeout_ms: u64) -> NetResult<Self> {
        if remote.port() == 0 {
            return Err(NetError::InvalidPort);
        }

        let socket = request_api::tcp_open()?;

        if let Err(error) = request_api::tcp_connect(socket, remote, timeout_ms) {
            let _ = request_api::tcp_abort(socket);
            return Err(error);
        }

        Ok(Self {
            socket,
            closed: false,
            read_timeout_ms: DEFAULT_IO_TIMEOUT_MS,
            write_timeout_ms: DEFAULT_IO_TIMEOUT_MS,
        })
    }

    pub fn set_read_timeout(&mut self, timeout_ms: u64) {
        self.read_timeout_ms = timeout_ms;
    }

    pub fn set_write_timeout(&mut self, timeout_ms: u64) {
        self.write_timeout_ms = timeout_ms;
    }

    pub fn close(mut self) -> NetResult<()> {
        if !self.closed {
            request_api::tcp_close(self.socket)?;
            self.closed = true;
        }

        Ok(())
    }
}

impl super::io::Read for TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> NetResult<usize> {
        if self.closed {
            return Err(NetError::Closed);
        }

        request_api::tcp_recv(self.socket, buf, self.read_timeout_ms)
    }
}

impl super::io::Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> NetResult<usize> {
        if self.closed {
            return Err(NetError::Closed);
        }

        request_api::tcp_send(self.socket, buf, self.write_timeout_ms)
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        if !self.closed {
            let _ = request_api::tcp_abort(self.socket);
            self.closed = true;
        }
    }
}
