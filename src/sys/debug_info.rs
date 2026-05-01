/// Prints an info, with a newline.
#[macro_export]
macro_rules! m_info {
    () => {
        let tick = $crate::syscall::get_tick();
        $crate::println!(
            "[I {:>3}.{:03}] ",
            tick / 1000,
            tick % 1000,
        );
    };
    ($fmt:expr) => {
        let tick = $crate::sys::syscall::get_tick();
        $crate::println!(
            "[I {:>3}.{:03}] {}",
            tick / 1000,
            tick % 1000,
            $fmt
        );
    };
    ($fmt:expr, $($arg:tt)*) => {
        let tick = $crate::sys::syscall::get_tick();
        ($crate::println!(concat!("[I {:>3}.{:03}] ", $fmt),
            tick / 1000,
            tick % 1000,
            $($arg)*));
    };
}

/// Prints a warning, with a newline.
#[macro_export]
macro_rules! m_warn {
    () => {
        let tick = $crate::sys::syscall::get_tick();
        $crate::println!(
            "[W {:>3}.{:03}] ",
            tick / 1000,
            tick % 1000,
        );
    };
    ($fmt:expr) => {
        let tick = $crate::sys::syscall::get_tick();
        $crate::println!(
            "[W {:>3}.{:03}] {}",
            tick / 1000,
            tick % 1000,
            $fmt
        );
    };
    ($fmt:expr, $($arg:tt)*) => {
        let tick = $crate::sys::syscall::get_tick();
        ($crate::println!(concat!("[W {:>3}.{:03}] ", $fmt),
            tick / 1000,
            tick % 1000,
            $($arg)*));
    };
}

// Prints a error, with a newline.
#[macro_export]
macro_rules! m_error {
    () => {
        let tick = $crate::sys::syscall::get_tick();
        $crate::println!(
            "[E {:>3}.{:03}] ",
            tick / 1000,
            tick % 1000,
        );
    };
    ($fmt:expr) => {
        let tick = $crate::sys::syscall::get_tick();
        $crate::println!(
            "[E {:>3}.{:03}] {}",
            tick / 1000,
            tick % 1000,
            $fmt
        );
    };
    ($fmt:expr, $($arg:tt)*) => {
        let tick = $crate::sys::syscall::get_tick();
        ($crate::println!(concat!("[E {:>3}.{:03}] ", $fmt),
            tick / 1000,
            tick % 1000,
            $($arg)*));
    };
}
