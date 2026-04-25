use crate::sys::synchronization::{IrqSafeNullLock, interface::Mutex};

pub mod queue_console;

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

pub fn read_line<const N: usize>() -> heapless::String<N> {
    let mut line = heapless::String::<N>::new();

    loop {
        let c = console().read_char();

        match c {
            '\r' | '\n' => {
                crate::print!("\r\n");
                return line;
            }

            '\x08' | '\x7f' => {
                if !line.is_empty() {
                    line.pop();
                    crate::print!("\x08 \x08");
                }
            }

            c if c.is_ascii_graphic() || c == ' ' => {
                if line.push(c).is_ok() {
                    crate::print!("{}", c);
                }
            }

            _ => {}
        }
    }
}
