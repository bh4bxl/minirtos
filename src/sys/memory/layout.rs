#![allow(dead_code)]
unsafe extern "C" {
    static __ram_start: u8;
    static __ram_end: u8;
    static __stack_pool_size: u8;
    static __stack_pool_end: u8;
    static __stack_pool_start: u8;
}

pub fn ram_start() -> usize {
    unsafe { &__ram_start as *const u8 as usize }
}

pub fn ram_end() -> usize {
    unsafe { &__ram_end as *const u8 as usize }
}

pub fn stack_pool_size() -> usize {
    unsafe { __stack_pool_size as usize }
}

pub fn stack_pool_end() -> usize {
    unsafe { &__stack_pool_end as *const u8 as usize }
}

pub fn stack_pool_start() -> usize {
    unsafe { &__stack_pool_start as *const u8 as usize }
}
