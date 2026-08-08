#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetError {
    NetworkDown,
    NotConfigured,
    InvalidAddress,
    InvalidPort,
    InvalidSocket,
    InvalidRequest,
    InvalidBuffer,
    NoSocketAvailable,
    NoRequestAvailable,
    NoBufferAvailable,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    AlreadyConnected,
    NotConnected,
    TimedOut,
    WouldBlock,
    Busy,
    Closed,
    IcmpBindFailed,
    IcmpSendFailed,
    QueueFull,
    Internal,
}

pub type NetResult<T> = core::result::Result<T, NetError>;
