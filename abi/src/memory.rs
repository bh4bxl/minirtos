#[repr(align(32))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Aligned32<T>(pub T);

impl<T> Aligned32<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

#[inline]
pub const fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    value & !(align - 1)
}

#[inline]
pub const fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedBufferHandle(u32);

impl SharedBufferHandle {
    pub const INVALID: Self = Self(u32::MAX);

    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SharedBufferInfo {
    pub handle: SharedBufferHandle,
    pub addr: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SharedBufferRef {
    pub handle: SharedBufferHandle,
    pub offset: u32,
    pub len: u32,
}
