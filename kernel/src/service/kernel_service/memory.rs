use minirtos_abi::{
    MESSAGE_ARG_COUNT, MessageData, SharedBufferHandle, SharedBufferInfo, SysError, TaskId,
    UserMutPtr, UserPtr,
};

use crate::{
    ipc::{shared_buffer_create, shared_buffer_destroy, shared_buffer_map, shared_buffer_unmap},
    synchronization::CriticalSection,
    sys::{read_user, write_user},
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

pub(super) fn handle_memory_read(
    cs: &CriticalSection,
    sender: TaskId,
    sub_op: u16,
    ptr: UserMutPtr<u8>,
    len: usize,
) -> Result<(), SysError> {
    if len != core::mem::size_of::<SharedBufferInfo>() {
        return Err(SysError::InvalidArgument);
    }

    let info_ptr = UserMutPtr::<SharedBufferInfo>::from_raw(ptr.raw());

    let request = read_user(UserPtr::<SharedBufferInfo>::from_raw(ptr.raw()))?;

    let info = match MemoryOp::try_from(sub_op)? {
        MemoryOp::SharedBufferAlloc => shared_buffer_create(cs, sender, request.size as usize)?,

        MemoryOp::SharedBufferMap => shared_buffer_map(cs, sender, request.handle)?,

        _ => return Err(SysError::InvalidArgument),
    };

    write_user(info_ptr, info)
}

pub(super) fn handle_memory(
    cs: &CriticalSection,
    sender: TaskId,
    sub_op: u16,
    args: &[u32; MESSAGE_ARG_COUNT],
) -> Result<MessageData, SysError> {
    let op = MemoryOp::try_from(sub_op)?;

    match op {
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

        _ => Err(SysError::InvalidArgument),
    }
}
