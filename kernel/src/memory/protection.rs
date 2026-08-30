use super::MemoryBlock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryAccess {
    ReadOnly,
    ReadWrite,
    ExecuteRead,
    ExecuteReadWrite,
    DeviceReadOnly,
    DeviceReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    mem_block: MemoryBlock,
    access: MemoryAccess,
}

impl MemoryRegion {
    pub const fn new(base: usize, size: usize, access: MemoryAccess) -> Self {
        Self {
            mem_block: MemoryBlock::new(base, size),
            access,
        }
    }

    pub const fn read_only(base: usize, size: usize) -> Self {
        Self::new(base, size, MemoryAccess::ReadOnly)
    }

    pub const fn read_write(base: usize, size: usize) -> Self {
        Self::new(base, size, MemoryAccess::ReadWrite)
    }

    pub const fn device_read_write(base: usize, size: usize) -> Self {
        Self::new(base, size, MemoryAccess::DeviceReadWrite)
    }
}
