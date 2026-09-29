use crate::{TaskId, UserMutPtr, UserPtr};

pub const MESSAGE_ARG_COUNT: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Service-specific operation.
    pub op: u32,

    /// Used by normal Data messages.
    pub args: [u32; MESSAGE_ARG_COUNT],
}

impl Default for ReceivedRequest {
    fn default() -> Self {
        Self {
            sender: TaskId::from_raw(0),
            op: 0,
            args: [0; MESSAGE_ARG_COUNT],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IpcMessageArgs {
    pub endpoint: EndpointHandle,
    pub message: UserPtr<MessageData>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IpcRecvArgs {
    pub endpoint: EndpointHandle,
    pub request: UserMutPtr<ReceivedRequest>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IpcCallArgs {
    pub endpoint: EndpointHandle,
    pub request: UserPtr<MessageData>,
    pub response: UserMutPtr<MessageData>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IpcCompleteArgs {
    pub endpoint: EndpointHandle,
    pub target: TaskId,
    pub response: UserPtr<MessageData>,
}

/// Syscall operation ID for IPC
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcOp {
    CreateEndpoint = 0,
    DestroyEndpoint = 1,

    /// Non-blocking send
    TrySend = 2,
    /// Non-blocking recv
    TryRecv = 3,

    /// Blocking send
    Send = 4,
    /// Blocking recv
    Recv = 5,

    /// Send request, and waiting server complete
    Call = 6,
    /// Server complete the call from client
    Complete = 7,
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
            6 => Ok(Self::Call),
            7 => Ok(Self::Complete),
            _ => Err(()),
        }
    }
}
