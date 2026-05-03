pub mod dma;
pub mod gpio;
pub mod input;
pub mod lcd;
pub mod spi;
pub mod uart;
pub mod wlan;

pub fn delay_ms(ms: u32) {
    // CPU HZ = 150MHz
    cortex_m::asm::delay(ms.saturating_mul(100_000));
}

pub fn delay_ns(ns: u32) {
    const NS_PER_LOOP: u32 = 10;

    let loops = (ns + NS_PER_LOOP - 1) / NS_PER_LOOP;

    cortex_m::asm::delay(loops.max(1));
}
