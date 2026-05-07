use rp235x_pac as pac;

use super::cyw43_regs::*;
use crate::{drivers::delay_ms, sys::device_driver::DevError};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum Func {
    Bus = 0,       // All SPI-specific registers
    BackPlane = 1, // Registers and memories belonging to other blocks in the chip (64 bytes max)
    Wlan = 2,      // DMA channel 1. WLAN packets up to 2048 bytes.
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum CoreId {
    WlanArm,
    Socram,
}

// BUS function registers
const SPI_BUS_CONTROL: u32 = 0x0000;
const SPI_RESPONSE_DELAY: u32 = 0x0001;
const SPI_STATUS_ENABLE: u32 = 0x0002;
const SPI_INTERRUPT_REGISTER: u32 = 0x0004; // 16 bits - Interrupt status
const SPI_INTERRUPT_ENABLE_REGISTER: u32 = 0x0006; // 16 bits - Interrupt mask
const SPI_READ_TEST_REGISTER: u32 = 0x0014; // 32 bits
const SPI_RESP_DELAY_F1: u32 = 0x001d; // 8 bits (corerev >= 3)

// BUS_CTRL bits
const WORD_LEN_32: u32 = 0x01; // 0/1 16/32 bit word length
const ENDIAN_BIG: u32 = 0x02; // 0/1 Little/Big Endian
#[allow(dead_code)]
const CLOCK_PHASE: u32 = 0x04; // 0/1 clock phase delay
#[allow(dead_code)]
const CLOCK_POLARITY: u32 = 0x08; // 0/1 Idle state clock polarity is low/high
const HIGH_SPEED_MODE: u32 = 0x10; // 1/0 High Speed mode / Normal mode
const INTERRUPT_POLARITY_HIGH: u32 = 0x20; //  1/0 Interrupt active polarity is high/low
const WAKE_UP: u32 = 0x80; // 0/1 Wake-up command from Host to WLAN

// STATUS_ENABLE bits
#[allow(dead_code)]
const STATUS_ENABLE: u32 = 0x01; // 1/0 Status sent/not sent to host after read/write
const INTR_WITH_STATUS: u32 = 0x02; // 0/1 Do-not / do-interrupt if status is sent
#[allow(dead_code)]
const RESP_DELAY_ALL: u32 = 0x04; // Applicability of resp delay to F1 or all func's read
#[allow(dead_code)]
const DWORD_PKT_LEN_EN: u32 = 0x08; // Packet len denoted in dwords instead of bytes
#[allow(dead_code)]
const CMD_ERR_CHK_EN: u32 = 0x20; // Command error check enable
#[allow(dead_code)]
const DATA_ERR_CHK_EN: u32 = 0x40; // Data error check enable

// SPI_INTERRUPT_REGISTER and SPI_INTERRUPT_ENABLE_REGISTER bits
const DATA_UNAVAILABLE: u16 = 0x0001; // Requested data not available; Clear by writing a "1"
const F2_F3_FIFO_RD_UNDERFLOW: u16 = 0x0002;
const F2_F3_FIFO_WR_OVERFLOW: u16 = 0x0004;
const COMMAND_ERROR: u16 = 0x0008; // Cleared by writing 1
const DATA_ERROR: u16 = 0x0010; // Cleared by writing 1
const F2_PACKET_AVAILABLE: u16 = 0x0020;
#[allow(dead_code)]
const F3_PACKET_AVAILABLE: u16 = 0x0040;
const F1_OVERFLOW: u16 = 0x0080; // Due to last write. Bkplane has pending write requests
#[allow(dead_code)]
const GSPI_PACKET_AVAILABLE: u16 = 0x0100;
#[allow(dead_code)]
const MISC_INTR1: u16 = 0x0200;
#[allow(dead_code)]
const MISC_INTR2: u16 = 0x0400;
#[allow(dead_code)]
const MISC_INTR3: u16 = 0x0800;
#[allow(dead_code)]
const MISC_INTR4: u16 = 0x1000;
#[allow(dead_code)]
const F1_INTR: u16 = 0x2000;
#[allow(dead_code)]
const F2_INTR: u16 = 0x4000;
#[allow(dead_code)]
const F3_INTR: u16 = 0x8000;

// The maximum block size for transfers on the bus.
const CYW43_BACKPLANE_READ_PAD_LEN_BYTES: usize = 16;
const CYW43_SPI_HEADER_SIZE: usize = (CYW43_BACKPLANE_READ_PAD_LEN_BYTES / 4) + 2;

// CYW43 Test Pattern
const TEST_PATTERN: u32 = 0xfeed_bead;

#[repr(C, align(4))]
pub(super) struct Cyw43Bus {
    spi: super::pio_spi::PioSpi,

    cur_backplane_window: u32,
    spi_header: [u32; CYW43_SPI_HEADER_SIZE],
}

#[allow(dead_code)]
impl Cyw43Bus {
    pub const fn new(spi: super::pio_spi::PioSpi) -> Self {
        Self {
            spi,
            cur_backplane_window: 0,
            spi_header: [0; CYW43_SPI_HEADER_SIZE],
        }
    }

    pub fn init(&mut self) -> Result<(), DevError> {
        if !self.check_alive()? {
            return Err(DevError::Unsupported);
        }

        self.set_control()?;

        self.set_alp()?;

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
    fn make_cmd(write: bool, incr: bool, func: Func, addr: u32, len: usize) -> u32 {
        ((write as u32) << 31)
            | ((incr as u32) << 30)
            | ((func as u32 & 0b11) << 28)
            | ((addr & 0x1_ffff) << 11)
            | (len as u32 & 0x7ff)
    }

    #[inline]
    fn read_reg_raw(&mut self, func: Func, addr: u32, size: usize) -> Result<u32, DevError> {
        let cmd = Self::make_cmd(false, true, func, addr, size);

        let padding = match func {
            Func::BackPlane => CYW43_BACKPLANE_READ_PAD_LEN_BYTES,
            _ => 0,
        };

        let mut buf32 = [0u32; (CYW43_BACKPLANE_READ_PAD_LEN_BYTES / 4) + 2];
        buf32[0] = cmd;

        let total = 8 + padding;

        let rx = unsafe { core::slice::from_raw_parts_mut(buf32.as_mut_ptr() as *mut u8, total) };

        self.spi.transfer(&[], rx)?;

        let index = if padding > 0 {
            (CYW43_BACKPLANE_READ_PAD_LEN_BYTES / 4) + 1
        } else {
            1
        };

        let res = buf32[index];

        defmt::debug!(
            "cyw43_read_reg_u{} {} 0x{:x}=0x{:x}",
            size * 8,
            match func {
                Func::Bus => "BUS_FUNCTION",
                Func::BackPlane => "BACKPLANE_FUNCTION",
                Func::Wlan => "wlan",
            },
            addr,
            res
        );

        Ok(res)
    }

    fn read_reg_u32(&mut self, func: Func, addr: u32) -> Result<u32, DevError> {
        self.read_reg_raw(func, addr, 4)
    }

    fn read_reg_u16(&mut self, func: Func, addr: u32) -> Result<u16, DevError> {
        Ok(self.read_reg_raw(func, addr, 2)? as u16)
    }

    pub fn read_reg_u8(&mut self, func: Func, addr: u32) -> Result<u8, DevError> {
        Ok(self.read_reg_raw(func, addr, 1)? as u8)
    }

    #[inline]
    fn write_reg_raw(
        &mut self,
        func: Func,
        addr: u32,
        val: u32,
        size: usize,
    ) -> Result<(), DevError> {
        let cmd = Self::make_cmd(true, true, func, addr, size);

        let mut tx = [0u8; 8];

        tx[0..4].copy_from_slice(&cmd.to_le_bytes());
        match size {
            1 => tx[4] = val as u8,
            2 => tx[4..6].copy_from_slice(&(val as u16).to_le_bytes()),
            4 => tx[4..8].copy_from_slice(&val.to_le_bytes()),
            _ => return Err(DevError::InvalidArg),
        }

        self.spi.transfer(&tx, &mut [])?;

        defmt::debug!(
            "cyw43_write_reg_u{} {} 0x{:x}=0x{:x}",
            size * 8,
            match func {
                Func::Bus => "BUS_FUNCTION",
                Func::BackPlane => "BACKPLANE_FUNCTION",
                Func::Wlan => "wlan",
            },
            addr,
            val
        );

        Ok(())
    }

    pub fn write_reg_u32(&mut self, func: Func, addr: u32, val: u32) -> Result<(), DevError> {
        self.write_reg_raw(func, addr, val, 4)
    }

    fn write_reg_u16(&mut self, func: Func, addr: u32, val: u16) -> Result<(), DevError> {
        self.write_reg_raw(func, addr, val as u32, 2)
    }

    pub fn write_reg_u8(&mut self, func: Func, addr: u32, val: u8) -> Result<(), DevError> {
        self.write_reg_raw(func, addr, val as u32, 1)
    }

    #[inline]
    fn swap16x2(val: u32) -> u32 {
        ((val & 0x00ff_00ff) << 8) | ((val & 0xff00_ff00) >> 8)
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
    fn write_reg_u32_swap(&mut self, func: Func, addr: u32, val: u32) -> Result<(), DevError> {
        let cmd = Self::make_cmd(true, true, func, addr, 4);

        let mut tx = [0u8; 8];

        tx[0..4].copy_from_slice(&Self::swap16x2(cmd).to_le_bytes());
        tx[4..8].copy_from_slice(&Self::swap16x2(val).to_le_bytes());

        self.spi.transfer(&tx, &mut [])?;

        defmt::debug!(
            "write_reg_u32_swap {} 0x{:08x}={:08x}",
            func as u32,
            addr,
            val
        );

        Ok(())
    }

    /// Read a block
    fn read_bytes(&mut self, func: Func, addr: u32, buf: &mut [u8]) -> Result<usize, DevError> {
        if buf.len() & 0b11 != 0 {
            return Err(DevError::InvalidArg);
        }

        let cmd = Self::make_cmd(false, true, func, addr, buf.len());
        let tx = cmd.to_le_bytes();
        self.spi.transfer(&tx, buf)?;

        Ok(buf.len())
    }

    /// Write a block
    pub fn write_bytes(&mut self, func: Func, addr: u32, buf: &[u8]) -> Result<usize, DevError> {
        if buf.len() > 64 {
            return Err(DevError::InvalidArg);
        }

        let aligned_len = (buf.len() + 3) & !3;
        let cmd = Self::make_cmd(true, true, func, addr, buf.len());

        let mut tx = [0u8; 4 + 64];

        tx[0..4].copy_from_slice(&cmd.to_le_bytes());
        tx[4..4 + buf.len()].copy_from_slice(buf);

        // padded bytes are already zero because tx is initialized with 0
        self.spi.transfer(&tx[..4 + aligned_len], &mut [])?;

        Ok(buf.len())
    }

    fn check_alive(&mut self) -> Result<bool, DevError> {
        for _ in 0..10 {
            if let Ok(res) = self.read_reg_u32_swap(Func::Bus, SPI_READ_TEST_REGISTER) {
                if res == TEST_PATTERN {
                    return Ok(true);
                }
            } else {
                return Err(DevError::Io);
            }
            delay_ms(1);
        }

        Ok(false)
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

        let ctrl = self.read_reg_u32(Func::Bus, SPI_BUS_CONTROL)?;
        defmt::info!("read SPI_BUS_CONTROL {:08x}", ctrl);

        self.write_reg_u8(
            Func::Bus,
            SPI_RESP_DELAY_F1,
            CYW43_BACKPLANE_READ_PAD_LEN_BYTES as u8,
        )?;

        // Check 32 bit mode
        let test = self.read_reg_u32(Func::Bus, SPI_READ_TEST_REGISTER)?;
        if test != TEST_PATTERN {
            return Err(DevError::Io);
        }

        // Make sure error interrupt bits are clear
        self.write_reg_u8(
            Func::Bus,
            SPI_INTERRUPT_REGISTER,
            (DATA_UNAVAILABLE | COMMAND_ERROR | DATA_ERROR | F1_OVERFLOW) as u8,
        )?;

        // Enable a selection of interrupts
        let cyw43_interrupts: u16 = F2_F3_FIFO_RD_UNDERFLOW
            | F2_F3_FIFO_WR_OVERFLOW
            | COMMAND_ERROR
            | DATA_ERROR
            | F2_PACKET_AVAILABLE
            | F1_OVERFLOW;
        self.write_reg_u16(Func::Bus, SPI_INTERRUPT_ENABLE_REGISTER, cyw43_interrupts)?;

        Ok(())
    }

    fn set_alp(&mut self) -> Result<(), DevError> {
        self.write_reg_u8(Func::BackPlane, SDIO_CHIP_CLOCK_CSR, SBSDIO_ALP_AVAIL_REQ)?;

        let mut checked = false;
        for _ in 0..10 {
            let reg = self.read_reg_u8(Func::BackPlane, SDIO_CHIP_CLOCK_CSR)?;
            defmt::info!("read SDIO_CHIP_CLOCK_CSR {:02x}", reg);
            if reg & SBSDIO_ALP_AVAIL != 0 {
                checked = true;
                break;
            }
            delay_ms(1);
        }

        if !checked {
            return Err(DevError::Io);
        }

        // clear request for ALP
        self.write_reg_u8(Func::BackPlane, SDIO_CHIP_CLOCK_CSR, 0)?;

        Ok(())
    }

    // ===== backplane stuff

    pub fn set_backplane_window(&mut self, addr: u32) -> Result<(), DevError> {
        let addr = addr & !BACKPLANE_ADDR_MASK;
        if addr == self.cur_backplane_window {
            return Ok(());
        }
        if (addr & 0xff00_0000) != (self.cur_backplane_window & 0xff00_0000) {
            self.write_reg_u8(
                Func::BackPlane,
                SDIO_BACKPLANE_ADDRESS_HIGH,
                (addr >> 24) as u8,
            )?;
        }
        if (addr & 0x00ff_0000) != (self.cur_backplane_window & 0x00ff_0000) {
            self.write_reg_u8(
                Func::BackPlane,
                SDIO_BACKPLANE_ADDRESS_MID,
                (addr >> 16) as u8,
            )?;
        }
        if (addr & 0x0000_ff00) != (self.cur_backplane_window & 0x0000_ff00) {
            self.write_reg_u8(
                Func::BackPlane,
                SDIO_BACKPLANE_ADDRESS_LOW,
                (addr >> 8) as u8,
            )?;
        }

        self.cur_backplane_window = addr;

        Ok(())
    }

    pub fn read_backplane(&mut self, addr: u32, size: usize) -> Result<u32, DevError> {
        self.set_backplane_window(addr)?;

        let mut addr = addr & BACKPLANE_ADDR_MASK;
        addr |= SBSDIO_SB_ACCESS_2_4B_FLAG;

        let val = self.read_reg_raw(Func::BackPlane, addr, size)?;

        self.set_backplane_window(CHIPCOMMON_BASE_ADDRESS)?;

        Ok(match size {
            1 => val & 0xff,
            4 => val,
            _ => return Err(DevError::InvalidArg),
        })
    }

    pub fn write_backplane(&mut self, addr: u32, val: u32, size: usize) -> Result<(), DevError> {
        if !matches!(size, 1 | 4) {
            return Err(DevError::InvalidArg);
        }

        self.set_backplane_window(addr)?;

        let mut local = addr & BACKPLANE_ADDR_MASK;
        local |= SBSDIO_SB_ACCESS_2_4B_FLAG;

        self.write_reg_raw(Func::BackPlane, local, val, size)?;

        self.set_backplane_window(CHIPCOMMON_BASE_ADDRESS)?;

        Ok(())
    }

    #[inline]
    pub fn get_core_address(&mut self, core_id: CoreId) -> u32 {
        match core_id {
            CoreId::WlanArm => WLAN_ARMCM3_BASE_ADDRESS + WRAPPER_REGISTER_OFFSET,
            CoreId::Socram => SOCSRAM_BASE_ADDRESS + WRAPPER_REGISTER_OFFSET,
        }
    }
}
