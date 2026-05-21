pub mod rp235x_i2c;

#[derive(Clone, Copy, Debug)]
pub struct I2cConfig {
    pub clk_sys: u32,
    pub baudrate: u32,
}

impl Default for I2cConfig {
    fn default() -> Self {
        Self {
            baudrate: 400_000,
            clk_sys: 150_000_000,
        }
    }
}

pub mod interface {
    use crate::sys::device_driver::DevError;

    pub trait I2cBus {
        fn write(&self, addr: u8, data: &[u8]) -> Result<(), DevError>;

        fn read(&self, addr: u8, data: &mut [u8]) -> Result<usize, DevError>;

        fn write_read(&self, addr: u8, w_data: &[u8], r_data: &mut [u8]) -> Result<(), DevError> {
            self.write(addr, w_data)?;
            self.read(addr, r_data)?;

            Ok(())
        }
    }
}
