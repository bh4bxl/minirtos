use minirtos_abi::{SharedBufferHandle, SharedBufferInfo, SysError, TaskId};

use crate::{
    MemoryBlock, sched,
    synchronization::{CriticalSection, CriticalSectionLock},
};

mod shared_buffer_pool;
mod shared_buffer_table;

pub(crate) use shared_buffer_pool::SharedBufferPool;
pub(crate) use shared_buffer_table::SharedBufferTable;

pub(crate) const MAX_SHARED_BUFFER_MAPPINGS: usize = 16;

pub(crate) struct SharedBuffer {
    handle: SharedBufferHandle,
    owner: TaskId,
    block: MemoryBlock,
    size: usize,
    mapped_tasks: [Option<TaskId>; MAX_SHARED_BUFFER_MAPPINGS],
}

impl SharedBuffer {
    pub(crate) const fn new(
        handle: SharedBufferHandle,
        owner: TaskId,
        block: MemoryBlock,
        size: usize,
    ) -> Self {
        Self {
            handle,
            owner,
            block,
            size,
            mapped_tasks: [None; MAX_SHARED_BUFFER_MAPPINGS],
        }
    }

    pub(crate) const fn handle(&self) -> SharedBufferHandle {
        self.handle
    }

    pub(crate) const fn owner(&self) -> TaskId {
        self.owner
    }

    pub(crate) const fn block(&self) -> MemoryBlock {
        self.block
    }

    pub(crate) const fn size(&self) -> usize {
        self.size
    }

    fn is_mapped_by(&self, task: TaskId) -> bool {
        self.mapped_tasks.iter().any(|entry| *entry == Some(task))
    }

    fn add_mapping(&mut self, task: TaskId) -> Result<(), SysError> {
        if task == self.owner || self.is_mapped_by(task) {
            return Err(SysError::AlreadyExists);
        }

        let slot = self
            .mapped_tasks
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(SysError::NoResource)?;

        *slot = Some(task);

        Ok(())
    }

    fn remove_mapping(&mut self, task: TaskId) -> Result<(), SysError> {
        let slot = self
            .mapped_tasks
            .iter_mut()
            .find(|entry| **entry == Some(task))
            .ok_or(SysError::NotFound)?;

        *slot = None;

        Ok(())
    }

    fn has_mappings(&self) -> bool {
        self.mapped_tasks.iter().any(Option::is_some)
    }
}

static SHARED_BUFFER_TABLE: CriticalSectionLock<SharedBufferTable> =
    CriticalSectionLock::new(SharedBufferTable::new());

static SHARED_BUFFER_POOL: CriticalSectionLock<SharedBufferPool> =
    CriticalSectionLock::new(SharedBufferPool::new());

#[inline]
fn align_up(value: usize, alignment: usize) -> Result<usize, SysError> {
    value
        .checked_add(alignment - 1)
        .map(|v| v & !(alignment - 1))
        .ok_or(SysError::InvalidArgument)
}

pub(crate) fn shared_buffer_create(
    cs: &CriticalSection,
    owner: TaskId,
    size: usize,
) -> Result<SharedBufferInfo, SysError> {
    let block = SHARED_BUFFER_POOL.lock(cs, |pool| pool.alloc(size))?;

    let handle = match SHARED_BUFFER_TABLE.lock(cs, |table| table.insert(owner, block, size)) {
        Ok(handle) => handle,
        Err(err) => {
            let _ = SHARED_BUFFER_POOL.lock(cs, |pool| pool.free(block));

            return Err(err);
        }
    };

    let map_result = sched::scheduler().add_task_rw_region(cs, owner, block.base(), block.size());

    if let Err(err) = map_result {
        let _ = SHARED_BUFFER_TABLE.lock(cs, |table| table.remove(owner, handle));

        let _ = SHARED_BUFFER_POOL.lock(cs, |pool| pool.free(block));

        return Err(err);
    }

    Ok(SharedBufferInfo {
        handle,
        addr: block.base() as u32,
        size: size as u32,
    })
}

pub(crate) fn shared_buffer_map(
    cs: &CriticalSection,
    task: TaskId,
    handle: SharedBufferHandle,
) -> Result<SharedBufferInfo, SysError> {
    let (block, size) = SHARED_BUFFER_TABLE.lock(cs, |table| {
        let buffer = table.get(handle)?;

        if buffer.owner() == task || buffer.is_mapped_by(task) {
            return Err(SysError::AlreadyExists);
        }

        Ok((buffer.block(), buffer.size()))
    })?;

    sched::scheduler().add_task_rw_region(cs, task, block.base(), block.size())?;

    let result = SHARED_BUFFER_TABLE.lock(cs, |table| {
        let buffer = table.get_mut(handle)?;
        buffer.add_mapping(task)
    });

    if let Err(err) = result {
        let _ = sched::scheduler().remove_task_region(cs, task, block.base(), block.size());

        return Err(err);
    }

    Ok(SharedBufferInfo {
        handle,
        addr: block.base() as u32,
        size: size as u32,
    })
}

pub(crate) fn shared_buffer_unmap(
    cs: &CriticalSection,
    task: TaskId,
    handle: SharedBufferHandle,
) -> Result<(), SysError> {
    let block = SHARED_BUFFER_TABLE.lock(cs, |table| {
        let buffer = table.get(handle)?;

        if buffer.owner() == task {
            return Err(SysError::InvalidState);
        }

        if !buffer.is_mapped_by(task) {
            return Err(SysError::NotFound);
        }

        Ok(buffer.block())
    })?;

    sched::scheduler().remove_task_region(cs, task, block.base(), block.size())?;

    SHARED_BUFFER_TABLE.lock(cs, |table| {
        let buffer = table.get_mut(handle)?;
        buffer.remove_mapping(task)
    })?;

    Ok(())
}

pub(crate) fn shared_buffer_destroy(
    cs: &CriticalSection,
    owner: TaskId,
    handle: SharedBufferHandle,
) -> Result<(), SysError> {
    let block = SHARED_BUFFER_TABLE.lock(cs, |table| {
        let buffer = table.get(handle)?;

        if buffer.owner() != owner {
            return Err(SysError::PermissionDenied);
        }

        if buffer.has_mappings() {
            return Err(SysError::Busy);
        }

        Ok(buffer.block())
    })?;

    sched::scheduler().remove_task_region(cs, owner, block.base(), block.size())?;

    let removed_block = SHARED_BUFFER_TABLE.lock(cs, |table| table.remove(owner, handle))?;

    debug_assert_eq!(removed_block.base(), block.base());
    debug_assert_eq!(removed_block.size(), block.size());

    SHARED_BUFFER_POOL.lock(cs, |pool| pool.free(removed_block))?;

    Ok(())
}
