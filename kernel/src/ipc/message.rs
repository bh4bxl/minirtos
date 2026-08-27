use minirtos_abi::{MessageData, UserMutPtr, UserPtr};

use crate::task::TaskId;

#[derive(Clone, Copy, Debug)]
pub(crate) enum MessagePayload {
    Data(MessageData),
    Write {
        op: u32,
        ptr: UserPtr<u8>,
        len: usize,
    },
    Read {
        op: u32,
        ptr: UserMutPtr<u8>,
        len: usize,
    },
}

/// Kernel-side IPC message.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Message {
    sender: TaskId,
    payload: MessagePayload,
}

impl Message {
    pub(crate) const fn data(sender: TaskId, data: MessageData) -> Self {
        Self {
            sender,
            payload: MessagePayload::Data(data),
        }
    }

    pub(crate) const fn write(sender: TaskId, op: u32, ptr: UserPtr<u8>, len: usize) -> Self {
        Self {
            sender,
            payload: MessagePayload::Write { op, ptr, len },
        }
    }

    pub(crate) const fn read(sender: TaskId, op: u32, ptr: UserMutPtr<u8>, len: usize) -> Self {
        Self {
            sender,
            payload: MessagePayload::Read { op, ptr, len },
        }
    }

    pub(crate) const fn sender(&self) -> TaskId {
        self.sender
    }

    pub(crate) const fn payload(&self) -> MessagePayload {
        self.payload
    }
}
