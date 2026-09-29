use minirtos_abi::MessageData;

use crate::task::TaskId;

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

    pub(crate) const fn data(&self) -> MessageData {
        self.data
    }
}
