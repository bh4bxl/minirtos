use heapless::Vec;

use super::super::SysError;

const MAX_FREE_BLOCKS: usize = 16;

#[derive(Clone, Copy, Debug)]
struct FreeBlock {
    start: usize,
    end: usize,
}

impl FreeBlock {
    const fn size(&self) -> usize {
        self.end - self.start
    }
}

pub struct StackPool {
    bottom: usize,
    top: usize,
    free_blocks: Vec<FreeBlock, MAX_FREE_BLOCKS>,
    initialized: bool,
}

impl StackPool {
    pub const fn empty() -> Self {
        Self {
            bottom: 0,
            top: 0,
            free_blocks: Vec::new(),
            initialized: false,
        }
    }

    fn init_once(&mut self) {
        if !self.initialized {
            self.bottom = super::layout::stack_pool_start();
            self.top = super::layout::stack_pool_end();

            assert_eq!(self.bottom & 0x7, 0, "stack pool bottom is not aligned");
            assert_eq!(self.top & 0x7, 0, "stack pool top is not aligned");
            assert!(self.bottom < self.top, "invalid stack pool range");

            self.free_blocks
                .push(FreeBlock {
                    start: self.bottom,
                    end: self.top,
                })
                .expect("failed to initialize stack pool");

            self.initialized = true;

            defmt::info!(
                "Stack pool: {:#010x}..{:#010x}, size={}",
                self.bottom,
                self.top,
                self.total()
            );
        }
    }

    pub fn alloc_words(&mut self, words: usize) -> Result<&'static mut [u32], SysError> {
        self.init_once();

        if words == 0 {
            return Err(SysError::NoMemory);
        }

        // Cortex-M exception stacks must remain 8-byte aligned.
        let words = (words + 1) & !1;
        let bytes = words
            .checked_mul(core::mem::size_of::<u32>())
            .ok_or(SysError::NoMemory)?;

        let index = self
            .free_blocks
            .iter()
            .position(|block| block.size() >= bytes)
            .ok_or(SysError::NoMemory)?;

        // Allocate from the high end of the free block because Cortex-M
        // task stacks grow downward.
        let block = self.free_blocks[index];
        let stack_start = block.end - bytes;

        if stack_start == block.start {
            self.free_blocks.remove(index);
        } else {
            self.free_blocks[index].end = stack_start;
        }

        debug_assert_eq!(stack_start & 0x7, 0);

        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                stack_start as *mut u32,
                words,
            ))
        }
    }

    pub fn free_words(&mut self, stack: &'static mut [u32]) {
        self.init_once();

        let start = stack.as_mut_ptr() as usize;
        let bytes = stack
            .len()
            .checked_mul(core::mem::size_of::<u32>())
            .expect("stack size overflow");
        let end = start.checked_add(bytes).expect("stack address overflow");

        assert!(!stack.is_empty(), "cannot free an empty stack");
        assert_eq!(start & 0x7, 0, "freed stack is not 8-byte aligned");
        assert_eq!(end & 0x7, 0, "freed stack end is not 8-byte aligned");

        assert!(
            start >= self.bottom && end <= self.top && start < end,
            "freed stack is outside stack pool: {:#010x}..{:#010x}",
            start,
            end
        );

        // Any overlap with an existing free block means either:
        // - double free
        // - corrupted stack metadata
        // - an invalid pointer was supplied
        for block in &self.free_blocks {
            assert!(
                end <= block.start || start >= block.end,
                "stack pool double free or overlapping free: \
                 new={:#010x}..{:#010x}, existing={:#010x}..{:#010x}",
                start,
                end,
                block.start,
                block.end
            );
        }

        let insert_at = self
            .free_blocks
            .iter()
            .position(|block| start < block.start)
            .unwrap_or(self.free_blocks.len());

        self.free_blocks
            .insert(insert_at, FreeBlock { start, end })
            .expect("stack pool free block table full");

        self.merge_adjacent_blocks();
    }

    fn merge_adjacent_blocks(&mut self) {
        let mut index = 0;

        while index + 1 < self.free_blocks.len() {
            let current = self.free_blocks[index];
            let next = self.free_blocks[index + 1];

            if current.end == next.start {
                self.free_blocks[index].end = next.end;
                self.free_blocks.remove(index + 1);
            } else {
                index += 1;
            }
        }
    }

    pub fn used(&self) -> usize {
        if !self.initialized {
            0
        } else {
            self.total() - self.free()
        }
    }

    pub fn free(&self) -> usize {
        if !self.initialized {
            return self.total();
        }

        self.free_blocks.iter().map(FreeBlock::size).sum()
    }

    pub fn total(&self) -> usize {
        super::layout::stack_pool_size()
    }
}
