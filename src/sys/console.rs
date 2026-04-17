use crate::sys::synchronization::{IrqSafeNullLock, interface::Mutex};

#[allow(dead_code)]
/// Console interface
pub mod interface {
    use core::fmt;

    /// Console write
    pub trait Write {
        /// Write a single character
        fn write_char(&self, c: char);

        /// Write a format string
        fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result;

        /// Block
        fn flush(&self);
    }

    /// Console read
    pub trait Read {
        /// Read a single character
        fn read_char(&self) -> char {
            ' '
        }

        /// Clear RX buffer
        fn clear_rx(&self) {}
    }

    pub trait All: Write + Read {}
}

/// A placeholder.
struct NullConsole;

impl interface::Write for NullConsole {
    fn write_char(&self, _c: char) {}

    fn write_fmt(&self, _args: core::fmt::Arguments) -> core::fmt::Result {
        core::fmt::Result::Ok(())
    }

    fn flush(&self) {}
}

impl interface::Read for NullConsole {}

impl interface::All for NullConsole {}

static NULL_CONSOLE: NullConsole = NullConsole {};

/// A reference to the global console.
static CURR_CONSOLE: IrqSafeNullLock<&'static (dyn interface::All + Sync)> =
    IrqSafeNullLock::new(&NULL_CONSOLE);

/// Register a new console.
pub fn register_console(new_console: &'static (dyn interface::All + Sync)) {
    CURR_CONSOLE.lock(|con| *con = new_console);
}

/// Return a reference to the currently registered console.
pub fn console() -> &'static dyn interface::All {
    CURR_CONSOLE.lock(|con| *con)
}
