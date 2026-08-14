#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! kerror {
    ($($arg:tt)*) => {
        defmt::error!($($arg)*)
    };
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kerror {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! kwarn {
    ($($arg:tt)*) => {
        defmt::warn!($($arg)*)
    };
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kwarn {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! kinfo {
    ($($arg:tt)*) => {
        defmt::info!($($arg)*)
    };
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kinfo {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! kdebug {
    ($($arg:tt)*) => {
        defmt::debug!($($arg)*)
    };
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kdebug {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! ktrace {
    ($($arg:tt)*) => {
        defmt::trace!($($arg)*)
    };
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! ktrace {
    ($($arg:tt)*) => {};
}
