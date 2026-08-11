use crate::sys::syscall;

pub(crate) struct SyscallConsole;

impl SyscallConsole {
    pub const fn new() -> Self {
        Self
    }
}

impl super::interface::Write for SyscallConsole {
    fn write_char(&self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        syscall::write(s.as_ptr(), s.len());
    }

    fn write_str(&self, s: &str) {
        if !s.is_empty() {
            syscall::write(s.as_ptr(), s.len());
        }
    }

    fn flush(&self) {}
}

impl super::interface::Read for SyscallConsole {
    fn try_read_char(&self) -> Option<char> {
        syscall::try_read_char()
    }

    fn read_char(&self) -> char {
        loop {
            if let Some(c) = self.try_read_char() {
                return c;
            }

            // ToDo:
            // Temporary polling implementation.
            // Replace with a blocking read syscall later.
            syscall::sleep_ms(1);
        }
    }
}

impl super::interface::All for SyscallConsole {}
