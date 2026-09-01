use minirtos_abi::SysError;

use crate::MemoryBlock;

const BLOCK_SIZE: usize = 256;
const BLOCK_COUNT: usize = 16;

#[repr(align(32))]
struct SharedBufferStorage {
    data: [u8; BLOCK_SIZE * BLOCK_COUNT],
}

static mut STORAGE: SharedBufferStorage = SharedBufferStorage {
    data: [0; BLOCK_SIZE * BLOCK_COUNT],
};

pub(crate) struct SharedBufferPool {
    used: [bool; BLOCK_COUNT],
}

impl SharedBufferPool {
    pub(crate) const fn new() -> Self {
        Self {
            used: [false; BLOCK_COUNT],
        }
    }

    pub(crate) fn alloc(&mut self, size: usize) -> Result<MemoryBlock, SysError> {
        if size == 0 || size > BLOCK_SIZE {
            return Err(SysError::InvalidArgument);
        }

        let index = self
            .used
            .iter()
            .position(|used| !*used)
            .ok_or(SysError::NoMemory)?;

        self.used[index] = true;

        let base = unsafe {
            core::ptr::addr_of!(STORAGE.data)
                .cast::<u8>()
                .add(index * BLOCK_SIZE) as usize
        };

        Ok(MemoryBlock::new(base, BLOCK_SIZE))
    }

    pub(crate) fn free(&mut self, block: MemoryBlock) -> Result<(), SysError> {
        let pool_base = unsafe { core::ptr::addr_of!(STORAGE.data) as usize };

        let offset = block
            .base()
            .checked_sub(pool_base)
            .ok_or(SysError::InvalidArgument)?;

        if offset % BLOCK_SIZE != 0 {
            return Err(SysError::InvalidArgument);
        }

        let index = offset / BLOCK_SIZE;

        if index >= BLOCK_COUNT {
            return Err(SysError::InvalidArgument);
        }

        if !self.used[index] {
            return Err(SysError::InvalidState);
        }

        self.used[index] = false;

        Ok(())
    }
}
