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
