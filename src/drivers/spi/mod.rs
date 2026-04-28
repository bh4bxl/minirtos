pub mod rp235x_pl022_spi;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum SpiMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum SpiBitOrder {
    MsbFirst,
    LsbFirst,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum DmaDir {
    Tx,
    Rx,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct SpiConfig {
    pub baudrate: u32,
    pub mode: SpiMode,
    pub bits_per_word: u8,
    pub bit_order: SpiBitOrder,
    pub clk_peri: u32,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            baudrate: 10_000_000,
            mode: SpiMode::Mode0,
            bits_per_word: 8,
            bit_order: SpiBitOrder::MsbFirst,
            clk_peri: 150_000_000,
        }
    }
}

#[allow(dead_code)]
pub mod interface {

    pub trait SpiBus {
        fn config(&self, config: &super::SpiConfig);

        /// Enable DMA
        fn enable_dma(&self, dir: super::DmaDir, enable: bool);

        /// Write without DMA
        fn write(&self, data: &[u8]) -> Result<usize, crate::sys::device_driver::DevError>;

        /// Write with DMA 8bit
        fn write_dma(&self, data: &[u8]) -> Result<usize, crate::sys::device_driver::DevError>;

        /// Write with DMA 16bit
        fn write_dma_u16(&self, data: &[u16])
        -> Result<usize, crate::sys::device_driver::DevError>;

        /// Reade without DMA
        fn read(&self, _data: &mut [u8]) -> Result<usize, crate::sys::device_driver::DevError> {
            Err(crate::sys::device_driver::DevError::Unsupported)
        }

        /// Reade without DMA
        fn read_dma(&self, _data: &mut [u8]) -> Result<usize, crate::sys::device_driver::DevError> {
            Err(crate::sys::device_driver::DevError::Unsupported)
        }
    }
}
