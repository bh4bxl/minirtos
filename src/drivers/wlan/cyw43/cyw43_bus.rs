use rp235x_pac as pac;

use crate::{drivers::delay_ms, sys::device_driver::DevError};

use super::{cyw43_consts::*, cyw43_regs::*};

pub(super) trait RegValue: Sized {
    const SIZE: usize;
    fn from_raw(v: u32) -> Self;
    fn into_raw(self) -> u32;
}

impl RegValue for u8 {
    const SIZE: usize = 1;
    fn from_raw(v: u32) -> Self {
        v as u8
    }
    fn into_raw(self) -> u32 {
        self as u32
    }
}

impl RegValue for u16 {
    const SIZE: usize = 2;
    fn from_raw(v: u32) -> Self {
        v as u16
    }
    fn into_raw(self) -> u32 {
        self as u32
    }
}

impl RegValue for u32 {
    const SIZE: usize = 4;
    fn from_raw(v: u32) -> Self {
        v
    }
    fn into_raw(self) -> u32 {
        self
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum Func {
    Bus = 0,       // All SPI-specific registers
    Backplane = 1, // Registers and memories belonging to other blocks in the chip (64 bytes max)
    Wlan = 2,      // DMA channel 1. WLAN packets up to 2048 bytes.
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum CoreId {
    WlanArm,
    Socram,
}

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
    pub(super) const fn new(spi: super::pio_spi::PioSpi) -> Self {
        Self {
            spi,
            cur_backplane_window: 0,
            spi_header: [0; CYW43_SPI_HEADER_SIZE],
        }
    }

    pub(super) fn init(&mut self) -> Result<(), DevError> {
        if !self.check_alive()? {
            return Err(DevError::Unsupported);
        }

        self.set_control()?;

        self.set_alp()?;

        Ok(())
    }

    pub(super) fn init_hw(
        &mut self,
        pio0: pac::PIO0,
        resets: &mut pac::RESETS,
    ) -> Result<(), DevError> {
        self.spi.init_hw(pio0, resets);

        Ok(())
    }

    pub(super) fn gpio_setup(&self) -> Result<(), DevError> {
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
            Func::Backplane => CYW43_BACKPLANE_READ_PAD_LEN_BYTES,
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
                Func::Backplane => "BACKPLANE_FUNCTION",
                Func::Wlan => "wlan",
            },
            addr,
            res
        );

        Ok(res)
    }

    pub(super) fn read_reg<T: RegValue>(&mut self, func: Func, addr: u32) -> Result<T, DevError> {
        let raw = self.read_reg_raw(func, addr, T::SIZE)?;
        Ok(T::from_raw(raw))
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
                Func::Backplane => "BACKPLANE_FUNCTION",
                Func::Wlan => "wlan",
            },
            addr,
            val
        );

        Ok(())
    }

    pub(super) fn write_reg<T: RegValue>(
        &mut self,
        func: Func,
        addr: u32,
        value: T,
    ) -> Result<(), DevError> {
        self.write_reg_raw(func, addr, value.into_raw(), T::SIZE)
    }

    // --=== Swap read/write for beginnings ===--

    #[inline]
    fn swap16x2(val: u32) -> u32 {
        ((val & 0x00ff_00ff) << 8) | ((val & 0xff00_ff00) >> 8)
    }

    /// Swap half word, used before config
    fn read_reg_u32_swap(&mut self, func: Func, addr: u32) -> Result<u32, DevError> {
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

    // --=== Read/write a block===--

    /// Read a block from CYW43 over SPI.
    ///
    /// The SPI transaction layout is:
    ///
    ///     [ 4-byte cmd ] [ padding/dummy bytes ] -> RX data
    ///
    /// The caller-provided `buf` is used directly as the RX buffer.
    ///
    /// CYW43 SPI requires the transfer length to be 4-byte aligned, so
    /// `aligned_len` may be larger than `read_len`. Any extra padded bytes
    /// at the end are ignored.
    ///
    /// Backplane accesses are limited to `CYW43_BUS_MAX_BLOCK_SIZE`,
    /// but WLAN/F2 transfers may be much larger.
    ///
    /// The entire transfer must happen within a single SPI transaction / CS
    /// assertion.
    pub(super) fn read_bytes(
        &mut self,
        func: Func,
        addr: u32,
        read_len: usize,
        buf: &mut [u8],
    ) -> Result<usize, DevError> {
        let aligned_len = super::align_up(read_len, 0b100);

        let pad = if func == Func::Backplane {
            CYW43_BACKPLANE_READ_PAD_LEN_BYTES
        } else {
            0
        };

        // [pad][4-byte cmd/resp][payload]
        let xfer_len = pad + 4 + aligned_len;

        if buf.len() < xfer_len {
            defmt::warn!(
                "CYW43: xfer_len {} is too big, buf len {}",
                xfer_len,
                buf.len()
            );
            return Err(DevError::InvalidArg);
        }

        buf[..xfer_len].fill(0);

        let cmd = Self::make_cmd(false, true, func, addr, read_len);

        buf[pad..pad + 4].copy_from_slice(&cmd.to_le_bytes());

        self.spi.transfer(&[], &mut buf[..xfer_len])?;

        let payload_offset = pad + 4;

        buf.copy_within(payload_offset..payload_offset + read_len, 0);

        Ok(read_len)
    }

    /// Write a block
    pub(super) fn write_bytes(
        &mut self,
        func: Func,
        addr: u32,
        buf: &[u8],
    ) -> Result<usize, DevError> {
        let aligned_len = super::align_up(buf.len(), 0b100);

        if aligned_len == 0 || aligned_len > 0x7f8 {
            defmt::warn!("buf len {} is too big", buf.len());
            return Err(DevError::InvalidArg);
        }

        if func == Func::Backplane && buf.len() > CYW43_BUS_MAX_BLOCK_SIZE {
            defmt::warn!("backplane buf len {} is too big", buf.len());
            return Err(DevError::InvalidArg);
        }

        if func == Func::Wlan {
            let mut f2_ready_attempts = 1000;

            while f2_ready_attempts > 0 {
                let bus_status = self.read_reg::<u32>(Func::Bus, SPI_STATUS_REGISTER)?;

                if (bus_status & STATUS_F2_RX_READY) != 0 {
                    break;
                }

                f2_ready_attempts -= 1;
            }

            if f2_ready_attempts == 0 {
                defmt::warn!("CYW43: F2 not ready");
                return Err(DevError::Timeout);
            }
        }

        let cmd = Self::make_cmd(true, true, func, addr, buf.len());
        let cmd_bytes = cmd.to_le_bytes();

        self.spi.start_spi_comms();

        self.spi.write_bytes(&cmd_bytes)?;
        self.spi.write_bytes(&buf)?;

        if aligned_len != buf.len() {
            let pad = [0u8; 3];
            self.spi.write_bytes(&pad[..aligned_len - buf.len()])?;
        }

        self.spi.stop_spi_comms();

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
        let ctrl = WORD_LENGTH_32
            | ENDIAN_BIG
            | HIGH_SPEED_MODE
            | INTERRUPT_POLARITY_HIGH
            | WAKE_UP
            | 0x4 << (8 * SPI_RESPONSE_DELAY)
            | INTR_WITH_STATUS << (8 * SPI_STATUS_ENABLE);

        defmt::info!("CYW43: setting SPI_BUS_CONTROL {:08x}", ctrl);
        self.write_reg_u32_swap(Func::Bus, SPI_BUS_CONTROL, ctrl)?;

        let ctrl = self.read_reg::<u32>(Func::Bus, SPI_BUS_CONTROL)?;
        defmt::info!("CYW43: read SPI_BUS_CONTROL {:08x}", ctrl);

        self.write_reg::<u8>(
            Func::Bus,
            SPI_RESP_DELAY_F1,
            CYW43_BACKPLANE_READ_PAD_LEN_BYTES as u8,
        )?;

        // Check 32 bit mode
        let test = self.read_reg::<u32>(Func::Bus, SPI_READ_TEST_REGISTER)?;
        if test != TEST_PATTERN {
            return Err(DevError::Io);
        }

        // Make sure error interrupt bits are clear
        self.write_reg::<u8>(
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
        self.write_reg::<u16>(Func::Bus, SPI_INTERRUPT_ENABLE_REGISTER, cyw43_interrupts)?;

        Ok(())
    }

    fn set_alp(&mut self) -> Result<(), DevError> {
        self.write_reg::<u8>(Func::Backplane, SDIO_CHIP_CLOCK_CSR, SBSDIO_ALP_AVAIL_REQ)?;

        let mut checked = false;
        for _ in 0..10 {
            let reg = self.read_reg::<u8>(Func::Backplane, SDIO_CHIP_CLOCK_CSR)?;
            defmt::info!("CYW43: read SDIO_CHIP_CLOCK_CSR {:02x}", reg);
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
        self.write_reg::<u8>(Func::Backplane, SDIO_CHIP_CLOCK_CSR, 0)?;

        Ok(())
    }

    // --=== Backplane Stuff ===--

    pub(super) fn set_backplane_window(&mut self, addr: u32) -> Result<(), DevError> {
        let addr = addr & !BACKPLANE_ADDR_MASK;
        if addr == self.cur_backplane_window {
            return Ok(());
        }
        if (addr & 0xff00_0000) != (self.cur_backplane_window & 0xff00_0000) {
            self.write_reg::<u8>(
                Func::Backplane,
                SDIO_BACKPLANE_ADDRESS_HIGH,
                (addr >> 24) as u8,
            )?;
        }
        if (addr & 0x00ff_0000) != (self.cur_backplane_window & 0x00ff_0000) {
            self.write_reg::<u8>(
                Func::Backplane,
                SDIO_BACKPLANE_ADDRESS_MID,
                (addr >> 16) as u8,
            )?;
        }
        if (addr & 0x0000_ff00) != (self.cur_backplane_window & 0x0000_ff00) {
            self.write_reg::<u8>(
                Func::Backplane,
                SDIO_BACKPLANE_ADDRESS_LOW,
                (addr >> 8) as u8,
            )?;
        }

        self.cur_backplane_window = addr;

        Ok(())
    }

    pub(super) fn read_backplane(&mut self, addr: u32, size: usize) -> Result<u32, DevError> {
        self.set_backplane_window(addr)?;

        let mut addr = addr & BACKPLANE_ADDR_MASK;
        addr |= SBSDIO_SB_ACCESS_2_4B_FLAG;

        let val = self.read_reg_raw(Func::Backplane, addr, size)?;

        self.set_backplane_window(CHIPCOMMON_BASE_ADDRESS)?;

        Ok(match size {
            1 => val & 0xff,
            4 => val,
            _ => return Err(DevError::InvalidArg),
        })
    }

    pub(super) fn write_backplane(
        &mut self,
        addr: u32,
        val: u32,
        size: usize,
    ) -> Result<(), DevError> {
        if !matches!(size, 1 | 4) {
            return Err(DevError::InvalidArg);
        }

        self.set_backplane_window(addr)?;

        let mut local = addr & BACKPLANE_ADDR_MASK;
        local |= SBSDIO_SB_ACCESS_2_4B_FLAG;

        self.write_reg_raw(Func::Backplane, local, val, size)?;

        self.set_backplane_window(CHIPCOMMON_BASE_ADDRESS)?;

        Ok(())
    }

    #[inline]
    pub(super) fn get_core_address(&mut self, core_id: CoreId) -> u32 {
        match core_id {
            CoreId::WlanArm => WLAN_ARMCM3_BASE_ADDRESS + WRAPPER_REGISTER_OFFSET,
            CoreId::Socram => SOCSRAM_BASE_ADDRESS + WRAPPER_REGISTER_OFFSET,
        }
    }

    pub(super) fn f2_ready(&mut self) -> Result<(), DevError> {
        for _ in 0..1000 {
            let reg = self.read_reg::<u32>(Func::Bus, SPI_STATUS_REGISTER)?;
            if (reg & STATUS_F2_RX_READY) != 0 {
                defmt::info!("CYW43: F2 ready");
                return Ok(());
            }
            delay_ms(1);
        }

        defmt::warn!("CYW43: F2 not ready");
        Err(DevError::Timeout)
    }

    pub(super) fn bus_sleep(&mut self, sleep: bool) -> Result<(), DevError> {
        if sleep {
            // TODO:
            // Enter KSO / low power mode
        } else {
            // TODO:
            // Force HT clock / wake WLAN core
        }

        Ok(())
    }

    pub(super) fn clear_sdio_pull_up(&mut self) -> Result<(), DevError> {
        self.write_reg::<u8>(Func::Backplane, SDIO_PULL_UP, 0)?;
        let _ = self.read_reg::<u8>(Func::Backplane, SDIO_PULL_UP)?;
        Ok(())
    }

    pub(super) fn clear_data_unavailable(&mut self) -> Result<(), DevError> {
        let int_status = self.read_reg::<u16>(Func::Bus, SPI_INTERRUPT_REGISTER)?;

        if int_status & DATA_UNAVAILABLE != 0 {
            self.write_reg::<u16>(Func::Bus, SPI_INTERRUPT_REGISTER, int_status)?;
        }

        Ok(())
    }
}
