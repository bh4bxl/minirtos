use crate::task::TaskId;

pub const MESSAGE_ARG_COUNT: usize = 4;

#[derive(Clone, Copy, Debug)]
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

/// Kernel-side IPC message.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Message {
    sender: TaskId,
    data: MessageData,
}

impl Message {
    pub(crate) const fn new(sender: TaskId, data: MessageData) -> Self {
        Self { sender, data }
    }

    pub(crate) const fn sender(&self) -> TaskId {
        self.sender
    }

    pub(crate) const fn data(&self) -> &MessageData {
        &self.data
    }

    pub(crate) const fn id(&self) -> u32 {
        self.data.id
    }
}
