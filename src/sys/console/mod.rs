use alloc::{
    string::{String, ToString},
    vec::Vec,
};

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

        /// Nonblocking read a single character
        fn try_read_char(&self) -> Option<char> {
            None
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

#[allow(dead_code)]
pub fn read_line<const N: usize>() -> String {
    let mut line = String::new();

    loop {
        let c = console().read_char();

        match c {
            '\r' | '\n' => {
                crate::print!("\r\n");
                return line;
            }

            '\x08' | '\x7f' => {
                if line.pop().is_some() {
                    crate::print!("\x08 \x08");
                }
            }

            c if c.is_ascii_graphic() || c == ' ' => {
                if line.len() < N {
                    line.push(c);
                    crate::print!("{}", c);
                }
            }

            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputKey {
    Character(char),
    Enter,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Unknown,
}

fn read_escape_sequence(con: &'static dyn interface::All) -> InputKey {
    match con.read_char() {
        'O' => match con.read_char() {
            'A' => InputKey::Up,
            'B' => InputKey::Down,
            'C' => InputKey::Right,
            'D' => InputKey::Left,
            'H' => InputKey::Home,
            'F' => InputKey::End,
            _ => InputKey::Unknown,
        },

        '[' => {
            let mut params = [0u8; 8];
            let mut len = 0usize;

            loop {
                let c = con.read_char();

                if ('@'..='~').contains(&c) {
                    return match c {
                        'A' => InputKey::Up,
                        'B' => InputKey::Down,
                        'C' => InputKey::Right,
                        'D' => InputKey::Left,
                        'H' => InputKey::Home,
                        'F' => InputKey::End,

                        '~' => match &params[..len] {
                            b"1" | b"7" => InputKey::Home,
                            b"3" => InputKey::Delete,
                            b"4" | b"8" => InputKey::End,
                            _ => InputKey::Unknown,
                        },

                        _ => InputKey::Unknown,
                    };
                }

                if len < params.len() && c.is_ascii() {
                    params[len] = c as u8;
                    len += 1;
                }
            }
        }

        _ => InputKey::Unknown,
    }
}

fn read_key() -> InputKey {
    let con = console();

    match con.read_char() {
        '\r' | '\n' => InputKey::Enter,
        '\x08' | '\x7f' => InputKey::Backspace,
        '\x1b' => read_escape_sequence(con),
        c if c.is_ascii_graphic() || c == ' ' => InputKey::Character(c),
        _ => InputKey::Unknown,
    }
}

fn move_cursor_left(count: usize) {
    if count > 0 {
        crate::print!("\x1b[{}D", count);
    }
}

fn move_cursor_right(count: usize) {
    if count > 0 {
        crate::print!("\x1b[{}C", count);
    }
}

fn redraw_line(prompt: &str, line: &str, cursor: usize) {
    crate::print!("\r\x1b[2K{}{}", prompt, line);

    let tail_len = line.len().saturating_sub(cursor);
    move_cursor_left(tail_len);
}

pub struct History {
    entries: Vec<String>,
    capacity: usize,
    browsing: Option<usize>,
    editing_backup: String,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
            browsing: None,
            editing_backup: String::new(),
        }
    }

    pub fn push(&mut self, line: &str) {
        if self.capacity == 0 {
            self.reset_navigation();
            return;
        }

        let line = line.trim();

        if line.is_empty() {
            self.reset_navigation();
            return;
        }

        if self.entries.last().map(String::as_str) == Some(line) {
            self.reset_navigation();
            return;
        }

        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }

        self.entries.push(line.to_string());
        self.reset_navigation();
    }

    fn previous(&mut self, current_line: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        let index = match self.browsing {
            None => {
                self.editing_backup.clear();
                self.editing_backup.push_str(current_line);
                self.entries.len() - 1
            }
            Some(0) => 0,
            Some(index) => index - 1,
        };

        self.browsing = Some(index);
        Some(self.entries[index].as_str())
    }

    pub fn next(&mut self) -> Option<&str> {
        let index = self.browsing?;

        if index + 1 < self.entries.len() {
            let next = index + 1;
            self.browsing = Some(next);
            Some(self.entries[next].as_str())
        } else {
            self.browsing = None;
            Some(self.editing_backup.as_str())
        }
    }

    fn reset_navigation(&mut self) {
        self.browsing = None;
        self.editing_backup.clear();
    }
}

pub fn read_line_with_history<const N: usize>(prompt: &str, history: &mut History) -> String {
    let mut line = String::with_capacity(N);
    let mut cursor = 0usize;

    crate::print!("{}", prompt);

    loop {
        match read_key() {
            InputKey::Enter => {
                move_cursor_right(line.len().saturating_sub(cursor));
                crate::print!("\r\n");
                history.reset_navigation();
                return line;
            }
            InputKey::Backspace => {
                if cursor == 0 {
                    continue;
                }

                cursor -= 1;
                line.remove(cursor);
                redraw_line(prompt, &line, cursor);
            }
            InputKey::Delete => {
                if cursor < line.len() {
                    line.remove(cursor);
                    redraw_line(prompt, &line, cursor);
                }
            }
            InputKey::Character(c) => {
                if line.len() > N {
                    continue;
                }

                if cursor == line.len() {
                    line.push(c);
                    cursor += 1;
                    crate::print!("{}", c);
                } else {
                    line.insert(cursor, c);
                    cursor += 1;
                    redraw_line(prompt, &line, cursor);
                }
            }
            InputKey::Up => {
                if let Some(selected) = history.previous(&line) {
                    line.clear();
                    line.push_str(selected);
                    cursor = line.len();
                    redraw_line(prompt, &line, cursor);
                }
            }
            InputKey::Down => {
                if let Some(selected) = history.next() {
                    line.clear();
                    line.push_str(selected);
                    cursor = line.len();
                    redraw_line(prompt, &line, cursor);
                }
            }
            InputKey::Left => {
                if cursor > 0 {
                    cursor -= 1;
                    move_cursor_left(1);
                }
            }
            InputKey::Right => {
                if cursor < line.len() {
                    cursor += 1;
                    move_cursor_right(1);
                }
            }
            InputKey::Home => {
                move_cursor_left(cursor);
                cursor = 0;
            }
            InputKey::End => {
                move_cursor_right(line.len() - cursor);
                cursor = line.len();
            }
            InputKey::Unknown => {}
        }
    }
}
