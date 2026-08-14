#![no_std]

mod logger;

pub fn init() {
    crate::kinfo!("Kernel initializing");
}
