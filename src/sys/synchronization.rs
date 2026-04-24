use core::cell::UnsafeCell;

/// Synchronization interfaces.
pub mod interface {
    /// Any object implementing this trait guarantees exclusive access to the data wrapped within
    /// the Mutex for the duration of the provided closure.
    pub trait Mutex {
        /// The type of data that is wrapped by this mutex.
        type Data;

        /// Locks the mutex and grants the closure temporary mutable access to the wrapped data.
        fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;
    }

    /// A reader-writer exclusion type.
    pub trait ReadWriteEx {
        /// The type of encapsulated data.
        type Data;

        /// Grants temporary mutable access to the encapsulated data.
        fn write<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;

        /// Grants temporary immutable access to the encapsulated data.
        fn read<'a, R>(&'a self, f: impl FnOnce(&'a Self::Data) -> R) -> R;
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

// unsafe impl<T> Send for NullLock<T> where T: ?Sized + Send {}
// unsafe impl<T> Sync for NullLock<T> where T: ?Sized + Send {}
unsafe impl<T> Send for NullLock<T> {}
unsafe impl<T> Sync for NullLock<T> {}

impl<T> NullLock<T> {
    /// Create an instance.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

impl<T> interface::Mutex for NullLock<T> {
    type Data = T;

    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        // In a real lock, there would be code encapsulating this line that ensures that this
        // mutable reference will ever only be given out once at a time.
        let data = unsafe { &mut *self.data.get() };

        f(data)
    }
}

pub struct IrqSafeNullLock<T>
where
    T: ?Sized,
{
    data: UnsafeCell<T>,
}

impl<T> IrqSafeNullLock<T> {
    /// Create an instance.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

//unsafe impl<T> Send for NullLock<T> where T: ?Sized + Send {}
//unsafe impl<T> Sync for NullLock<T> where T: ?Sized + Sync {}
unsafe impl<T: ?Sized> Send for IrqSafeNullLock<T> {}
unsafe impl<T: ?Sized> Sync for IrqSafeNullLock<T> {}

impl<T> interface::Mutex for IrqSafeNullLock<T> {
    type Data = T;

    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        cortex_m::interrupt::free(|_| f(unsafe { &mut *self.data.get() }))
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

impl<T> interface::ReadWriteEx for InitStateLock<T> {
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

pub struct CriticalSection(());

pub fn critical_section<R>(f: impl FnOnce(&CriticalSection) -> R) -> R {
    cortex_m::interrupt::free(|_| {
        let cs = CriticalSection(());
        f(&cs)
    })
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
