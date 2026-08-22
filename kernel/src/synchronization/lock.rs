use core::cell::UnsafeCell;

use crate::arch;

struct IrqGuard {
    state: arch::IrqState,
}

impl IrqGuard {
    #[inline]
    fn new() -> Self {
        Self {
            state: arch::disable_interrupts(),
        }
    }
}

impl Drop for IrqGuard {
    #[inline]
    fn drop(&mut self) {
        arch::restore_interrupts(self.state);
    }
}

/// A pseudo-lock with no actual synchronization.
/// Safe only when external execution context guarantees exclusive access
pub struct NullLock<T>
where
    T: ?Sized,
{
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for NullLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for NullLock<T> where T: ?Sized + Send {}

impl<T> NullLock<T> {
    /// Create an instance.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

impl<T> super::interface::Lock for NullLock<T> {
    type Data = T;

    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        // In a real lock, there would be code encapsulating this line that ensures that this
        // mutable reference will ever only be given out once at a time.
        let data = unsafe { &mut *self.data.get() };

        f(data)
    }
}

/// IRQ-safe lock for single-core kernel execution.
///
/// This does not provide cross-core exclusion.
pub struct IrqLock<T>
where
    T: ?Sized,
{
    data: UnsafeCell<T>,
}

impl<T> IrqLock<T> {
    /// Create an instance.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

unsafe impl<T> Send for IrqLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for IrqLock<T> where T: ?Sized + Sync {}

impl<T> super::interface::Lock for IrqLock<T> {
    type Data = T;

    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        let _guard = IrqGuard::new();

        f(unsafe { &mut *self.data.get() })
    }
}

/// A pseudo-lock that is RW during the single-core kernel init phase and RO afterwards.
pub struct InitStateLock<T>
where
    T: ?Sized,
{
    data: UnsafeCell<T>,
}

impl<T> InitStateLock<T> {
    /// Create an instance.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

unsafe impl<T> Send for InitStateLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for InitStateLock<T> where T: ?Sized + Send {}

impl<T> super::interface::ReadWriteEx for InitStateLock<T> {
    type Data = T;

    fn write<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        let data = unsafe { &mut *self.data.get() };

        f(data)
    }

    fn read<'a, R>(&'a self, f: impl FnOnce(&'a Self::Data) -> R) -> R {
        let data = unsafe { &*self.data.get() };

        f(data)
    }
}

pub struct CriticalSectionLock<T: ?Sized> {
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for CriticalSectionLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for CriticalSectionLock<T> where T: ?Sized + Send {}

impl<T> CriticalSectionLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> CriticalSectionLock<T> {
    pub fn lock<R>(&self, _cs: &CriticalSection, f: impl FnOnce(&mut T) -> R) -> R {
        f(unsafe { &mut *self.data.get() })
    }

    /// Only for PendSV
    pub unsafe fn lock_unchecked<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(unsafe { &mut *self.data.get() })
    }
}

pub(crate) struct CriticalSection(());

pub(crate) fn critical_section<R>(f: impl FnOnce(&CriticalSection) -> R) -> R {
    let _guard = IrqGuard::new();

    let cs = CriticalSection(());
    f(&cs)
}
