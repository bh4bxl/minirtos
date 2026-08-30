use super::MemoryBlock;

unsafe extern "C" {
    static __flash_start: u8;
    static __flash_end: u8;

    static __ram_start: u8;
    static __ram_end: u8;

    static __heap_start: u8;
    static __heap_end: u8;

    static __stack_pool_end: u8;
    static __stack_pool_start: u8;

    static __kernel_stack_reserve: u8;
}

pub fn flash_block() -> MemoryBlock {
    let start = core::ptr::addr_of!(__flash_start) as usize;
    let end = core::ptr::addr_of!(__flash_end) as usize;

    MemoryBlock::new(start, end - start)
}

pub fn ram_block() -> MemoryBlock {
    let start = core::ptr::addr_of!(__ram_start) as usize;
    let end = core::ptr::addr_of!(__ram_end) as usize;

    MemoryBlock::new(start, end - start)
}

pub fn heap_block() -> MemoryBlock {
    let start = core::ptr::addr_of!(__heap_start) as usize;
    let end = core::ptr::addr_of!(__heap_end) as usize;

    MemoryBlock::new(start, end - start)
}

pub fn stack_pool_block() -> MemoryBlock {
    let start = core::ptr::addr_of!(__stack_pool_start) as usize;
    let end = core::ptr::addr_of!(__stack_pool_end) as usize;

    MemoryBlock::new(start, end - start)
}

pub fn reserve_size() -> usize {
    unsafe { &__kernel_stack_reserve as *const _ as usize }
}
