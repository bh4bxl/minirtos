#![allow(dead_code)]
unsafe extern "C" {
    static __ram_start: u8;
    static __ram_end: u8;

    static __heap_start: u8;
    static __heap_end: u8;

    static __stack_pool_end: u8;
    static __stack_pool_start: u8;

    static __kernel_stack_reserve: u8;
}

pub fn ram_start() -> usize {
    unsafe { &__ram_start as *const u8 as usize }
}

pub fn ram_end() -> usize {
    unsafe { &__ram_end as *const u8 as usize }
}

pub fn heap_start() -> usize {
    unsafe { &__heap_start as *const u8 as usize }
}

pub fn heap_end() -> usize {
    unsafe { &__heap_end as *const u8 as usize }
}

pub fn heap_size() -> usize {
    heap_end() - heap_start()
}

pub fn stack_pool_end() -> usize {
    unsafe { &__stack_pool_end as *const u8 as usize }
}

pub fn stack_pool_start() -> usize {
    unsafe { &__stack_pool_start as *const u8 as usize }
}

pub fn stack_pool_size() -> usize {
    stack_pool_end() - stack_pool_start()
}

pub fn reserve_size() -> usize {
    unsafe { &__kernel_stack_reserve as *const _ as usize }
}
