pub mod gpio;
pub mod lcd;
pub mod spi;
pub mod uart;

pub fn delay_ms(ms: u32) {
    // CPU HZ = 150MHz
    cortex_m::asm::delay(ms.saturating_mul(100_000));
}
