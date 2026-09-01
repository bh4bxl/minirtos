use minirtos_abi::{SharedBufferHandle, SysError, TaskId};

use crate::MemoryBlock;

use super::SharedBuffer;

const MAX_SHARED_BUFFERS: usize = 16;

pub(crate) struct SharedBufferTable {
    entries: [Option<SharedBuffer>; MAX_SHARED_BUFFERS],
}

impl SharedBufferTable {
    pub(crate) const fn new() -> Self {
        Self {
            entries: [const { None }; MAX_SHARED_BUFFERS],
        }
    }

    pub(crate) fn insert(
        &mut self,
        owner: TaskId,
        block: MemoryBlock,
        size: usize,
    ) -> Result<SharedBufferHandle, SysError> {
        let index = self
            .entries
            .iter()
            .position(Option::is_none)
            .ok_or(SysError::NoResource)?;

        let handle = SharedBufferHandle::from_raw(index as u32);

        self.entries[index] = Some(SharedBuffer::new(handle, owner, block, size));

        Ok(handle)
    }

    pub(crate) fn get(&self, handle: SharedBufferHandle) -> Result<&SharedBuffer, SysError> {
        self.entries
            .get(handle.raw() as usize)
            .and_then(Option::as_ref)
            .ok_or(SysError::NotFound)
    }

    pub(crate) fn get_mut(
        &mut self,
        handle: SharedBufferHandle,
    ) -> Result<&mut SharedBuffer, SysError> {
        self.entries
            .get_mut(handle.raw() as usize)
            .and_then(Option::as_mut)
            .ok_or(SysError::NotFound)
    }

    pub(crate) fn remove(
        &mut self,
        owner: TaskId,
        handle: SharedBufferHandle,
    ) -> Result<MemoryBlock, SysError> {
        let entry = self
            .entries
            .get_mut(handle.raw() as usize)
            .ok_or(SysError::NotFound)?;

        let buffer = entry.as_ref().ok_or(SysError::NotFound)?;

        if buffer.owner() != owner {
            return Err(SysError::PermissionDenied);
        }

        let block = buffer.block();

        *entry = None;

        Ok(block)
    }
}
