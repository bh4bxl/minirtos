pub mod uart_client;
pub mod uart_service;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataBits {
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopBits {
    One = 1,
    Two = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    None = 0,
    Odd = 1,
    Even = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UartConfig {
    pub clock_hz: u32,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            clock_hz: 150_000_000,
            baud_rate: 115_200,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
        }
    }
}

pub mod interface {

    pub trait UartDriver {
        type Error;

        fn init(&mut self) -> Result<(), Self::Error>;

        fn config(&mut self, config: &super::UartConfig) -> Result<(), Self::Error>;

        fn try_read_byte(&self) -> Result<Option<u8>, Self::Error>;

        fn write_byte(&self, byte: u8) -> Result<(), Self::Error>;

        fn write_buf(&self, buf: &[u8]) -> Result<usize, Self::Error>;
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UartOp {
    WriteByte = 0,
    TryReadByte = 1,
    Write = 2,
    Read = 3,
    Config = 4,
}

impl TryFrom<u32> for UartOp {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::WriteByte),
            1 => Ok(Self::TryReadByte),
            2 => Ok(Self::Write),
            3 => Ok(Self::Read),
            4 => Ok(Self::Config),
            _ => Err(()),
        }
    }
}
