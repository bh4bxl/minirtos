use core::marker::PhantomData;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserPtr<T> {
    raw: u32,
    _marker: PhantomData<*const T>,
}

impl<T> UserPtr<T> {
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn is_null(self) -> bool {
        self.raw == 0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserMutPtr<T> {
    raw: u32,
    _marker: PhantomData<*mut T>,
}

impl<T> UserMutPtr<T> {
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }

    pub const fn is_null(self) -> bool {
        self.raw == 0
    }
}
