/// Synchronization interfaces.

/// Any object implementing this trait guarantees exclusive access to the data wrapped within
/// the Mutex for the duration of the provided closure.
pub(crate) trait Lock {
    /// The type of data that is wrapped by this mutex.
    type Data;

    /// Locks the mutex and grants the closure temporary mutable access to the wrapped data.
    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;
}

/// A reader-writer exclusion type.
pub(crate) trait ReadWriteEx {
    /// The type of encapsulated data.
    type Data;

    /// Grants temporary mutable access to the encapsulated data.
    fn write<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;

    /// Grants temporary immutable access to the encapsulated data.
    fn read<'a, R>(&'a self, f: impl FnOnce(&'a Self::Data) -> R) -> R;
}
