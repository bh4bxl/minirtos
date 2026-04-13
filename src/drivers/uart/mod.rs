pub mod rp235x_pl011_uart;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub baudrate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: Parity,
    pub clock_hz: u32,
    pub eable_irq: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baudrate: 115_200,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
            clock_hz: 150_000_000,
            eable_irq: true,
        }
    }
}

pub mod interface {
    pub trait Uart {
        /// Configure the uart
        fn config(&self, config: &super::Config);
    }
}
