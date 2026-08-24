use crate::{UserMutPtr, UserPtr};

pub const MESSAGE_ARG_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MessageData {
    pub id: u32,
    pub args: [u32; MESSAGE_ARG_COUNT],
}

impl MessageData {
    pub const fn new(id: u32, args: [u32; MESSAGE_ARG_COUNT]) -> Self {
        Self { id, args }
    }
}

impl Default for MessageData {
    fn default() -> Self {
        Self {
            id: 0,
            args: [0; MESSAGE_ARG_COUNT],
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ReceivedMessage {
    pub sender: u32,
    pub data: MessageData,
}

impl Default for ReceivedMessage {
    fn default() -> Self {
        Self {
            sender: 0,
            data: MessageData::default(),
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
    pub message: UserMutPtr<ReceivedMessage>,
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
            _ => Err(()),
        }
    }
}
