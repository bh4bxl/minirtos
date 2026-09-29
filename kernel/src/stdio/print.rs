use core::fmt;

use crate::stdio::console;

pub fn _print(args: fmt::Arguments) {
    console::console().write_fmt(args).unwrap();
}

/// Prints without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::stdio::print::_print(format_args!($($arg)*)));
}

/// Prints with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n\r")
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\n\r"))
    };
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n\r"), $($arg)*));
}
