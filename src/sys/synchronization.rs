use core::cell::UnsafeCell;

/// Synchronization interfaces.
pub mod interface {
    pub trait Mutex {
        /// The type of data that is wrapped by this mutex.
        type Data;

        /// Locks the mutex and grants the closure temporary mutable access to the wrapped data.
        fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;
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
