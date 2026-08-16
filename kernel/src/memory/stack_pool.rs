use heapless::Vec;

use crate::{SysError, arch, synchronization::IrqLock};

pub const MAX_FREE_BLOCKS: usize = 32;

#[derive(Clone, Copy, Debug)]
struct FreeBlock {
    start: usize,
    end: usize,
}

impl FreeBlock {
    #[inline]
    const fn size(self) -> usize {
        self.end - self.start
    }
}

#[derive(Debug)]
pub(crate) struct StackRegion {
    start: usize,
    size: usize,
}

impl StackRegion {
    #[inline]
    pub const fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub const fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub const fn end(&self) -> usize {
        self.start + self.size
    }

    #[inline]
    pub const fn top(&self) -> *mut u8 {
        self.end() as *mut u8
    }

    #[inline]
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.start as *mut u8
    }

    #[inline]
    pub unsafe fn as_mut_slice(&mut self) -> &'static mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.start as *mut u8, self.size) }
    }

    #[inline]
    pub unsafe fn as_mut_words(&mut self) -> &'static mut [u32] {
        debug_assert_eq!(self.start & 0x3, 0);
        debug_assert_eq!(self.size & 0x3, 0);

        unsafe {
            core::slice::from_raw_parts_mut(
                self.start as *mut u32,
                self.size / core::mem::size_of::<u32>(),
            )
        }
    }

    pub fn fill_u32(&mut self, value: u32) {
        assert_eq!(self.start & 0x3, 0);
        assert_eq!(self.size & 0x3, 0);

        let words = self.size / core::mem::size_of::<u32>();

        unsafe {
            let ptr = self.start as *mut u32;

            for i in 0..words {
                ptr.add(i).write(value);
            }
        }
    }

    pub fn count_prefix_u32(&self, value: u32) -> usize {
        assert_eq!(self.start & 0x3, 0);
        assert_eq!(self.size & 0x3, 0);

        let words = self.size / core::mem::size_of::<u32>();
        let ptr = self.start as *const u32;

        let mut count = 0;

        unsafe {
            while count < words && ptr.add(count).read() == value {
                count += 1;
            }
        }

        count
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
        if self.initialized {
            return;
        }

        self.bottom = super::layout::stack_pool_start();
        self.top = super::layout::stack_pool_end();

        let align = arch::stack_alignment();

        assert!(
            align.is_power_of_two(),
            "invalid stack alignment: {}",
            align
        );

        assert_eq!(
            self.bottom & (align - 1),
            0,
            "stack pool bottom is not aligned"
        );

        assert_eq!(self.top & (align - 1), 0, "stack pool top is not aligned");

        assert!(self.bottom < self.top, "invalid stack pool range");

        self.free_blocks
            .push(FreeBlock {
                start: self.bottom,
                end: self.top,
            })
            .expect("failed to initialize stack pool");

        self.initialized = true;

        crate::kinfo!(
            "Stack pool: {:#010x}..{:#010x}, size={}",
            self.bottom,
            self.top,
            self.total_bytes()
        );
    }

    pub fn alloc(&mut self, size: usize) -> Result<StackRegion, SysError> {
        self.init_once();

        if size == 0 {
            return Err(SysError::InvalidArgument);
        }

        let align = arch::stack_alignment();

        let size = align_up(size, align).ok_or(SysError::NoMemory)?;

        let index = self
            .free_blocks
            .iter()
            .position(|block| block.size() >= size)
            .ok_or(SysError::NoMemory)?;

        let block = self.free_blocks[index];

        // Allocate from the high end of the block.
        //
        // This is convenient for downward-growing stacks and keeps the
        // stack top naturally aligned.
        let start = block.end - size;

        if start == block.start {
            self.free_blocks.remove(index);
        } else {
            self.free_blocks[index].end = start;
        }

        debug_assert_eq!(start & (align - 1), 0);
        debug_assert_eq!((start + size) & (align - 1), 0);

        Ok(StackRegion { start, size })
    }

    pub fn free(&mut self, region: StackRegion) {
        self.init_once();

        let start = region.start;
        let end = region.end();

        assert!(region.size != 0, "cannot free an empty stack region");

        assert!(
            start >= self.bottom && end <= self.top && start < end,
            "freed stack is outside stack pool: {:#010x}..{:#010x}",
            start,
            end
        );

        let align = arch::stack_alignment();

        assert_eq!(start & (align - 1), 0, "freed stack start is not aligned");

        assert_eq!(end & (align - 1), 0, "freed stack end is not aligned");

        // Any overlap with an existing free block means either:
        // - double free
        // - corrupted stack metadata
        // - invalid region
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

    pub fn used_bytes(&self) -> usize {
        self.total_bytes() - self.free_bytes()
    }

    pub fn free_bytes(&self) -> usize {
        if !self.initialized {
            return self.total_bytes();
        }

        self.free_blocks.iter().map(|block| block.size()).sum()
    }

    pub fn total_bytes(&self) -> usize {
        super::layout::stack_pool_size()
    }

    pub fn largest_free_block(&self) -> usize {
        if !self.initialized {
            return self.total_bytes();
        }

        self.free_blocks
            .iter()
            .map(|block| block.size())
            .max()
            .unwrap_or(0)
    }

    pub fn free_block_count(&self) -> usize {
        if !self.initialized {
            1
        } else {
            self.free_blocks.len()
        }
    }
}

#[inline]
fn align_up(value: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());

    value
        .checked_add(align - 1)
        .map(|value| value & !(align - 1))
}

pub(crate) static STACK_POOL: IrqLock<StackPool> = IrqLock::new(StackPool::empty());
