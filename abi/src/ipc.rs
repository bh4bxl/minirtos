use crate::{TaskId, UserMutPtr, UserPtr};

pub const MESSAGE_ARG_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum IpcMessageKind {
    /// Inline control message.
    Data = 0,

    /// Client provides a readable buffer to the service.
    Write = 1,

    /// Client provides a writable buffer to the service.
    Read = 2,
}

impl TryFrom<u32> for IpcMessageKind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Data),
            1 => Ok(Self::Write),
            2 => Ok(Self::Read),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MessageData {
    pub op: u32,
    pub args: [u32; MESSAGE_ARG_COUNT],
}

impl MessageData {
    pub const fn new(op: u32, args: [u32; MESSAGE_ARG_COUNT]) -> Self {
        Self { op, args }
    }
}

impl Default for MessageData {
    fn default() -> Self {
        Self {
            op: 0,
            args: [0; MESSAGE_ARG_COUNT],
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct IpcWriteArgs {
    pub endpoint: EndpointHandle,
    pub op: u32,
    pub ptr: UserPtr<u8>,
    pub len: usize,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct IpcReadArgs {
    pub endpoint: EndpointHandle,
    pub op: u32,
    pub ptr: UserMutPtr<u8>,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct EndpointHandle(u32);

impl EndpointHandle {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ReceivedRequest {
    pub sender: TaskId,

    pub kind: IpcMessageKind,

    /// Service-specific operation.
    pub op: u32,

    /// Used by normal Data messages.
    pub args: [u32; MESSAGE_ARG_COUNT],

    /// Used by Read/Write requests.
    pub ptr: u32,
    pub len: usize,
}

impl Default for ReceivedRequest {
    fn default() -> Self {
        Self {
            sender: TaskId::from_raw(0),
            kind: IpcMessageKind::Data,
            op: 0,
            args: [0; MESSAGE_ARG_COUNT],
            ptr: 0,
            len: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IpcSendArgs {
    pub endpoint: EndpointHandle,
    pub message: UserPtr<MessageData>,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IpcRecvArgs {
    pub endpoint: EndpointHandle,
    pub request: UserMutPtr<ReceivedRequest>,
}

/// Syscall operation ID for IPC
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcOp {
    CreateEndpoint = 0,
    DestroyEndpoint = 1,
    TrySend = 2,
    TryRecv = 3,
    Send = 4,
    Recv = 5,
    Write = 6,
    Read = 7,
    Complete = 8,
}

impl TryFrom<u32> for IpcOp {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CreateEndpoint),
            1 => Ok(Self::DestroyEndpoint),
            2 => Ok(Self::TrySend),
            3 => Ok(Self::TryRecv),
            4 => Ok(Self::Send),
            5 => Ok(Self::Recv),
            6 => Ok(Self::Write),
            7 => Ok(Self::Read),
            8 => Ok(Self::Complete),
            _ => Err(()),
        }
    }
}
