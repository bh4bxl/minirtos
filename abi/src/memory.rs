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
