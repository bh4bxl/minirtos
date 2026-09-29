/// Write functions.
pub trait IoWrite {
    /// Write a single character.
    fn write_char(&self, c: char);

    /// Write formatted output.
    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result;

    /// Block until all buffered output has reached the device.
    fn flush(&self);
}

/// Read functions.
pub trait IoRead {
    /// Block until a character is available.
    fn read_char(&self) -> char;

    /// Try to read a character without blocking.
    fn try_read_char(&self) -> Option<char>;

    /// Clear pending receive data.
    fn clear_input(&self);
}

/// Trait alias for a full-fledged console.
pub trait IoAll: IoWrite + IoRead {}

impl<T> IoAll for T where T: IoWrite + IoRead + ?Sized {}
