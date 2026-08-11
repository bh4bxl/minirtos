use core::fmt;

use super::console;

#[doc(hidden)]
pub fn _printk(args: fmt::Arguments) {
    console::console().write_fmt(args).unwrap();
}

/// Prints without a newline.
#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ($crate::sys::print::_printk(format_args!($($arg)*)));
}

/// Prints with a newline.
#[macro_export]
macro_rules! printkln {
    () => {
        $crate::printk!("\r\n")
    };
    ($fmt:expr) => {
        $crate::printk!(concat!($fmt, "\r\n"))
    };
    ($fmt:expr, $($arg:tt)*) => ($crate::printk!(concat!($fmt, "\r\n"), $($arg)*));
}
