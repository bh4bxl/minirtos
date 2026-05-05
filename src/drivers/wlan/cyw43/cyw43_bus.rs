use rp235x_pac as pac;

use crate::{drivers::delay_ms, sys::device_driver::DevError};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum Func {
    Bus = 0,       // All SPI-specific registers
    BackPlane = 1, // Registers and memories belonging to other blocks in the chip (64 bytes max)
    Wlan = 2,      // DMA channel 1. WLAN packets up to 2048 bytes.
}

// BUS function registers
const SPI_BUS_CONTROL: u32 = 0x0000;
const SPI_RESPONSE_DELAY: u32 = 0x0001;
const SPI_STATUS_ENABLE: u32 = 0x0002;
const SPI_READ_TEST_REGISTER: u32 = 0x0014;

// BUS_CTRL bits
const WORD_LEN_32: u32 = 0x01; // 0/1 16/32 bit word length
const ENDIAN_BIG: u32 = 0x02; // 0/1 Little/Big Endian
const CLOCK_PHASE: u32 = 0x04; // 0/1 clock phase delay
const CLOCK_POLARITY: u32 = 0x08; // 0/1 Idle state clock polarity is low/high
const HIGH_SPEED_MODE: u32 = 0x10; // 1/0 High Speed mode / Normal mode
const INTERRUPT_POLARITY_HIGH: u32 = 0x20; //  1/0 Interrupt active polarity is high/low
const WAKE_UP: u32 = 0x80; // 0/1 Wake-up command from Host to WLAN

// STATUS_ENABLE bits
const STATUS_ENABLE: u32 = 0x01; // 1/0 Status sent/not sent to host after read/write
const INTR_WITH_STATUS: u32 = 0x02; // 0/1 Do-not / do-interrupt if status is sent
const RESP_DELAY_ALL: u32 = 0x04; // Applicability of resp delay to F1 or all func's read
const DWORD_PKT_LEN_EN: u32 = 0x08; // Packet len denoted in dwords instead of bytes
const CMD_ERR_CHK_EN: u32 = 0x20; // Command error check enable
const DATA_ERR_CHK_EN: u32 = 0x40; // Data error check enable

// The maximum block size for transfers on the bus.
const CYW43_BACKPLANE_READ_PAD_LEN_BYTES: usize = 16;

// CYW43 Test Pattern
const TEST_PATTERN: u32 = 0xfeed_bead;

pub(super) struct Cyw43Bus {
    spi: super::pio_spi::PioSpi,
}

#[allow(dead_code)]
impl Cyw43Bus {
    pub const fn new(spi: super::pio_spi::PioSpi) -> Self {
        Self { spi }
    }

    pub fn init(&mut self) -> Result<(), DevError> {
        if !self.check_alive() {
            return Err(DevError::Unsupported);
        }

        self.set_control()?;

        Ok(())
    }

    pub fn init_hw(&mut self, pio0: pac::PIO0, resets: &mut pac::RESETS) -> Result<(), DevError> {
        self.spi.init_hw(pio0, resets);

        Ok(())
    }

    pub fn gpio_setup(&self) -> Result<(), DevError> {
        self.spi.gpio_setup();

        Ok(())
    }

    /// Command
    /// Bit layout (32-bit, MSB first):
    ///   [31]     = 0:read  1:write
    ///   [30]     = increment address
    ///   [29:28]  = function (0=BUS, 1=BACKPLANE, 2=WLAN)
    ///   [27:11]  = register address (17 bits)
    ///   [10:0]   = byte length (11 bits)
    /// fn make_cmd(&self, write: bool, inc: bool, func: u32, addr: u32, size: u32) -> u32
    #[inline]
    fn make_cmd(write: bool, incr: bool, func: Func, addr: u32, len: u32) -> u32 {
        ((write as u32) << 31)
            | ((incr as u32) << 30)
            | ((func as u32 & 0b11) << 28)
            | ((addr & 0x1_ffff) << 11)
            | (len & 0x7ff)
    }

    #[inline]
    fn read_reg(&self, func: Func, addr: u32, size: u32) -> Result<u32, DevError> {
        let index = (CYW43_BACKPLANE_READ_PAD_LEN_BYTES / 4) + 1 + 1;
        // ToDo:
        Ok(size)
    }

    pub fn read_reg_u32(&mut self, func: Func, addr: u32) -> Result<u32, DevError> {
        let cmd = Self::make_cmd(false, true, func, addr, 4);
        let tx = cmd.to_le_bytes();
        let mut rx = [0u8; 8];

        self.spi.transfer(&tx, &mut rx)?;

        Ok(u32::from_le_bytes(rx[4..8].try_into().unwrap()))
    }

    pub fn write_reg_u32(&mut self, func: Func, addr: u32, val: u32) -> Result<(), DevError> {
        let cmd = Self::make_cmd(true, true, func, addr, 4);
        let mut tx = [0u8; 8];

        tx[0..4].copy_from_slice(&cmd.to_le_bytes());
        tx[4..8].copy_from_slice(&val.to_le_bytes());

        self.spi.transfer(&tx, &mut [])?;

        Ok(())
    }

    #[inline]
    fn swap16x2(value: u32) -> u32 {
        ((value & 0x00ff_00ff) << 8) | ((value & 0xff00_ff00) >> 8)
    }

    /// Swap half word, used before config
    pub fn read_reg_u32_swap(&mut self, func: Func, addr: u32) -> Result<u32, DevError> {
        let cmd = Self::make_cmd(false, true, func, addr, 4);

        let tx = Self::swap16x2(cmd).to_le_bytes();
        let mut rx = [0u8; 8];

        self.spi.transfer(&tx, &mut rx)?;

        let raw = u32::from_le_bytes(rx[4..8].try_into().unwrap());

        Ok(Self::swap16x2(raw))
    }

    /// Swap half word, used before config
    fn write_reg_u32_swap(&mut self, func: Func, addr: u32, value: u32) -> Result<(), DevError> {
        let cmd = Self::make_cmd(true, true, func, addr, 4);

        let mut tx = [0u8; 8];

        tx[0..4].copy_from_slice(&Self::swap16x2(cmd).to_le_bytes());
        tx[4..8].copy_from_slice(&Self::swap16x2(value).to_le_bytes());

        let mut du = [0u8; 12];
        self.spi.transfer(&tx, &mut du)?;

        Ok(())
    }

    /// Read a block
    fn read_bytes(&mut self, func: Func, addr: u32, buf: &mut [u8]) -> Result<usize, DevError> {
        if buf.len() & 0b11 != 0 {
            return Err(DevError::InvalidArg);
        }

        let cmd = Self::make_cmd(false, true, func, addr, buf.len() as u32);
        let tx = cmd.to_be_bytes();
        self.spi.transfer(&tx, buf)?;

        Ok(buf.len())
    }

    /// Write a block
    fn write_bytes(&mut self, func: Func, addr: u32, buf: &[u8]) -> Result<usize, DevError> {
        if buf.len() & 0b11 != 0 || buf.len() > 64 {
            return Err(DevError::InvalidArg);
        }

        let cmd = Self::make_cmd(true, true, func, addr, buf.len() as u32);
        let mut tx = [0u8; 4 + 64];
        tx[0..4].copy_from_slice(&cmd.to_be_bytes());
        tx[4..4 + buf.len()].copy_from_slice(buf);
        self.spi.transfer(&tx, &mut [])?;

        Ok(buf.len())
    }

    fn check_alive(&mut self) -> bool {
        for i in 0..10 {
            if let Ok(res) = self.read_reg_u32_swap(Func::Bus, SPI_READ_TEST_REGISTER) {
                if res == TEST_PATTERN {
                    return true;
                }
            }
            delay_ms(1);
        }

        false
    }

    fn set_control(&mut self) -> Result<(), DevError> {
        let ctrl = WORD_LEN_32
            | ENDIAN_BIG
            | HIGH_SPEED_MODE
            | INTERRUPT_POLARITY_HIGH
            | WAKE_UP
            | 0x4 << (8 * SPI_RESPONSE_DELAY)
            | INTR_WITH_STATUS << (8 * SPI_STATUS_ENABLE);

        defmt::info!("setting SPI_BUS_CONTROL {:08x}", ctrl);
        self.write_reg_u32_swap(Func::Bus, SPI_BUS_CONTROL, ctrl)?;

        let ctrl = self.read_reg_u32(Func::Bus, SPI_BUS_CONTROL).unwrap();
        defmt::info!("read SPI_BUS_CONTROL {:08x}", ctrl);
        let ctrl = self
            .read_reg_u32(Func::Bus, SPI_READ_TEST_REGISTER)
            .unwrap();
        defmt::info!("read SPI_READ_TEST_REGISTER {:08x}", ctrl);

        Ok(())
    }
}
