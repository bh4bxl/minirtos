use super::super::SysError;

pub struct StackPool {
    bottom: usize,
    current: usize,
    initialized: bool,
}

impl StackPool {
    pub const fn empty() -> Self {
        Self {
            bottom: 0,
            current: 0,
            initialized: false,
        }
    }

    fn init_once(&mut self) {
        if !self.initialized {
            self.bottom = super::layout::stack_pool_start();
            self.current = super::layout::stack_pool_end();

            self.initialized = true;

            defmt::info!(
                "Stack pool: {:#010x}..{:#010x}, size={}",
                self.bottom,
                self.current,
                super::layout::stack_pool_size()
            );
        }
    }

    pub fn alloc_words(&mut self, words: usize) -> Result<&'static mut [u32], SysError> {
        self.init_once();

        let words = (words + 1) & !1;
        let bytes = words * core::mem::size_of::<u32>();

        let new_current = self.current.checked_sub(bytes).ok_or(SysError::NoMemory)? & !7;

        if new_current < self.bottom {
            return Err(SysError::NoMemory);
        }

        self.current = new_current;

        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                new_current as *mut u32,
                words,
            ))
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
            super::layout::stack_pool_size()
        } else {
            self.current - self.bottom
        }
    }

    pub fn total(&self) -> usize {
        super::layout::stack_pool_size()
    }
}
