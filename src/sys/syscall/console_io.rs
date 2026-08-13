use super::{
    super::console::{
        self,
        interface::{Read, Write},
    },
    SyscallId, syscall,
};

pub(crate) struct SyscallConsole;

impl SyscallConsole {
    pub const fn new() -> Self {
        Self
    }
}

impl console::interface::Write for SyscallConsole {
    fn write_char(&self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        write(s.as_ptr(), s.len());
    }

    fn write_str(&self, s: &str) {
        if !s.is_empty() {
            write(s.as_ptr(), s.len());
        }
    }

    fn flush(&self) {}
}

impl console::interface::Read for SyscallConsole {
    fn try_read_char(&self) -> Option<char> {
        try_read_char()
    }

    fn read_char(&self) -> char {
        loop {
            if let Some(c) = self.try_read_char() {
                return c;
            }

            // ToDo:
            // Temporary polling implementation.
            // Replace with a blocking read syscall later.
            super::sleep_ms(1);
        }
    }
}

impl console::interface::All for SyscallConsole {}

/// Write string to console
pub fn write(buf: *const u8, len: usize) -> usize {
    let ret = syscall::<{ SyscallId::Write as u8 }>(&[buf as u32, len as u32]);
    ret as usize
}

/// Try read a char
pub fn try_read_char() -> Option<char> {
    match syscall::<{ SyscallId::TryReadChar as u8 }>(&[]) {
        u32::MAX => None,
        c => char::from_u32(c),
    }
}

/// Read line
pub fn read_line<'a>(buf: &'a mut [u8]) -> &'a str {
    SyscallConsole::new().read_line(buf)
}

pub fn _print(args: core::fmt::Arguments) {
    SyscallConsole::new().write_fmt(args).unwrap();
}

/// Prints without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::sys::syscall::_print(format_args!($($arg)*)));
}

/// Prints with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\r\n")
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\r\n"))
    };
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\r\n"), $($arg)*));
}
