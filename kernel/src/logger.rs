#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kerror {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kwarn {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kinfo {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! kdebug {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! ktrace {
    ($($arg:tt)*) => {};
}
