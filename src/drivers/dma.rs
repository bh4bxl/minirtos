#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DmaChannles {
    Spi0Tx = 0,
    Spi0Rx = 1,
    Spi1Tx = 2,
    Spi1Rx = 3,
    Uart0Tx = 4,
    Uart0Rx = 5,
    Uart1Tx = 6,
    Uart1Rx = 7,
    I2c0Tx = 8,
    I2c0Rx = 9,
    I2c1Tx = 10,
    I2c1Rx = 11,
}
