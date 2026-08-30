mod heap;
mod layout;
mod protection;
mod stack_pool;

pub use heap::init_heap;
pub(crate) use layout::{flash_block, ram_block, stack_pool_block};
pub use protection::{MemoryAccess, MemoryRegion};
pub(crate) use stack_pool::{STACK_POOL, StackRegion};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryBlock {
    base: usize,
    size: usize,
}

impl MemoryBlock {
    pub const fn new(base: usize, size: usize) -> Self {
        Self { base, size }
    }

    pub const fn base(self) -> usize {
        self.base
    }

    pub const fn size(self) -> usize {
        self.size
    }

    pub fn end(self) -> usize {
        self.base + self.size
    }
}
