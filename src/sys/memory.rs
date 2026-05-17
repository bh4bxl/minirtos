unsafe extern "C" {
    static __ram_start: u8;
    static __ram_end: u8;
}

#[allow(dead_code)]
pub fn ram_start() -> usize {
    unsafe { &__ram_start as *const u8 as usize }
}

pub fn ram_end() -> usize {
    unsafe { &__ram_end as *const u8 as usize }
}
