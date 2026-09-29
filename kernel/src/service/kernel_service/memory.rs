use minirtos_abi::{MESSAGE_ARG_COUNT, MessageData, SharedBufferHandle, SysError, TaskId};

use crate::{
    ipc::{shared_buffer_create, shared_buffer_destroy, shared_buffer_map, shared_buffer_unmap},
    synchronization::CriticalSection,
};

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryOp {
    SharedBufferAlloc = 0,
    SharedBufferFree = 1,
    SharedBufferMap = 2,
    SharedBufferUnmap = 3,
}

impl TryFrom<u16> for MemoryOp {
    type Error = SysError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SharedBufferAlloc),
            1 => Ok(Self::SharedBufferFree),
            2 => Ok(Self::SharedBufferMap),
            3 => Ok(Self::SharedBufferUnmap),
            _ => Err(SysError::NotSupported),
        }
    }
}

pub(super) fn handle_memory(
    cs: &CriticalSection,
    sender: TaskId,
    sub_op: u16,
    args: &[u32; MESSAGE_ARG_COUNT],
) -> Result<MessageData, SysError> {
    let op = MemoryOp::try_from(sub_op)?;

    match op {
        MemoryOp::SharedBufferAlloc => {
            let size = args[0] as usize;

            let info = shared_buffer_create(cs, sender, size)?;

            Ok(MessageData::new(
                0,
                [info.handle.raw(), info.addr, info.size, 0],
            ))
        }

        MemoryOp::SharedBufferMap => {
            let handle = SharedBufferHandle::from_raw(args[0]);

            let info = shared_buffer_map(cs, sender, handle)?;

            Ok(MessageData::new(
                0,
                [info.handle.raw(), info.addr, info.size, 0],
            ))
        }

        MemoryOp::SharedBufferFree => {
            let handle = SharedBufferHandle::from_raw(args[0]);

            shared_buffer_destroy(cs, sender, handle)?;

            Ok(MessageData::default())
        }

        MemoryOp::SharedBufferUnmap => {
            let handle = SharedBufferHandle::from_raw(args[0]);

            shared_buffer_unmap(cs, sender, handle)?;

            Ok(MessageData::default())
        }
    }
}
