use crate::{
    drivers::{
        delay_ms,
        wlan::cyw43::{
            cyw43_bus::CoreId,
            cyw43_ioctl::Interface,
            cyw43_sdpcm::{SDPCM_HEADER_LEN, SdpcmOp, WlcCmd},
        },
    },
    sys::device_driver::DevError,
};

use super::{Cyw43Inner, cyw43_bus::Func, cyw43_regs::*};

pub(crate) static CYW43_FW: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/w43439A0_7_95_49_00_combined.bin"
));
pub(super) const CYW43_FW_LEN: usize = 224190;

pub(super) static WIFI_NVRAM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/wifi_nvram_43439.bin"));
pub(super) const WIFI_NVRAM_LEN: usize = 984;

const CYW43_BUS_MAX_BLOCK_SIZE: usize = 64;
#[allow(dead_code)]
const CYW43_BACKPLANE_READ_PAD_LEN_BYTES: usize = 16;

const CYW43_RAM_SIZE: usize = 512 * 1024;

const CLM_CHUNK_LEN: usize = 1024;
const DLOAD_HANDLER_VER: u16 = 1 << 12;
const DL_BEGIN: u16 = 2;
const DL_END: u16 = 4;
const DL_TYPE_CLM: u16 = 2;

impl Cyw43Inner {
    fn disable_device_core(&mut self, core_id: CoreId, _core_halt: bool) -> Result<(), DevError> {
        let base = self.bus.get_core_address(core_id);
        self.bus.read_backplane(base + AI_RESETCTRL_OFFSET, 1)?;
        let reg = self.bus.read_backplane(base + AI_RESETCTRL_OFFSET, 1)?;
        if reg & AIRC_RESET != 0 {
            return Ok(());
        }
        Err(DevError::Io)
    }

    fn reset_device_core(&mut self, core_id: CoreId, core_halt: bool) -> Result<(), DevError> {
        self.disable_device_core(core_id, core_halt)?;

        let base = self.bus.get_core_address(core_id);
        let halt = if core_halt { SICF_CPUHALT } else { 0 };

        self.bus
            .write_backplane(base + AI_IOCTRL_OFFSET, SICF_FGC | SICF_CLOCK_EN | halt, 1)?;
        self.bus.read_backplane(base + AI_IOCTRL_OFFSET, 1)?;
        self.bus.write_backplane(base + AI_RESETCTRL_OFFSET, 0, 1)?;

        delay_ms(1);

        self.bus
            .write_backplane(base + AI_IOCTRL_OFFSET, SICF_CLOCK_EN | halt, 1)?;
        self.bus.read_backplane(base + AI_IOCTRL_OFFSET, 1)?;

        delay_ms(1);

        Ok(())
    }

    fn device_core_is_up(&mut self, core_id: CoreId) -> Result<(), DevError> {
        let base = self.bus.get_core_address(core_id);

        let reg = self.bus.read_backplane(base + AI_IOCTRL_OFFSET, 1)?;
        if (reg & (SICF_FGC | SICF_CLOCK_EN)) != SICF_CLOCK_EN {
            defmt::warn!("core not up: ioctrl={:08x}", reg);
            return Err(DevError::Io);
        }

        let reg = self.bus.read_backplane(base + AI_RESETCTRL_OFFSET, 1)?;
        if reg & AIRC_RESET != 0 {
            defmt::warn!("core not up: resetctrl={:08x}", reg);
            return Err(DevError::Io);
        }

        defmt::info!("CYW43: core is up");
        Ok(())
    }

    fn check_valid_chipset_firmware(&self, fw: &[u8], fw_size: usize) -> Result<(), DevError> {
        let fw_end_len = 800;

        if fw_size < fw_end_len || fw_size > fw.len() {
            defmt::warn!("invalid firmware size {}", fw_size);
            return Err(DevError::InvalidArg);
        }

        let b = &fw[fw_size - fw_end_len..fw_size];

        let fw_end = fw_end_len - 16; // skip DVID trailer

        let trail_len = u16::from_le_bytes([b[fw_end - 2], b[fw_end - 1]]) as usize;

        if trail_len < 500 && b[fw_end - 3] == 0 {
            for i in 80..trail_len {
                let pos = fw_end - 3 - i;

                if pos + 9 <= b.len() && &b[pos..pos + 9] == b"Version: " {
                    defmt::info!("CYW43: valid firmware found");
                    return Ok(());
                }
            }
        }

        Err(DevError::Io)
    }

    fn download_resource(&mut self, addr: u32, data: &[u8]) -> Result<(), DevError> {
        if data.len() & 0b11 != 0 {
            defmt::warn!("invalid resource size {}", data.len());
            return Err(DevError::InvalidArg);
        }

        for offset in (0..data.len()).step_by(CYW43_BUS_MAX_BLOCK_SIZE) {
            let end = core::cmp::min(offset + CYW43_BUS_MAX_BLOCK_SIZE, data.len());
            let chunk = &data[offset..end];

            let dest_addr = addr + offset as u32;

            if ((dest_addr & BACKPLANE_ADDR_MASK) as usize + chunk.len())
                > (BACKPLANE_ADDR_MASK as usize + 1)
            {
                defmt::warn!("invalid dest_addr 0x{:08x}", dest_addr);
                return Err(DevError::InvalidArg);
            }

            self.bus.set_backplane_window(dest_addr)?;

            let mut local_addr = dest_addr & BACKPLANE_ADDR_MASK;
            local_addr |= SBSDIO_SB_ACCESS_2_4B_FLAG;

            self.bus.write_bytes(Func::Backplane, local_addr, chunk)?;
        }

        Ok(())
    }

    pub(crate) fn download_firmware(
        &mut self,
        fw: &[u8],
        fw_size: usize,
        nvram: &[u8],
        _nvram_size: usize,
    ) -> Result<(), DevError> {
        defmt::info!("CYW43: downloading firmware");

        self.disable_device_core(CoreId::WlanArm, false)?;
        self.disable_device_core(CoreId::Socram, false)?;
        self.reset_device_core(CoreId::Socram, false)?;

        // this is 4343x specific stuff: Disable remap for SRAM_3
        self.bus.write_backplane(SOCSRAM_BANKX_INDEX, 0x03, 4)?;
        self.bus.write_backplane(SOCSRAM_BANKX_PDA, 0x00, 4)?;

        // Check that valid chipset firmware exists at the given source address.
        self.check_valid_chipset_firmware(fw, fw_size)?;

        // Download the main WiFi firmware blob to the 43xx device.
        defmt::info!("CYW43: main firmware: {}", fw.len());
        self.download_resource(0x0000_0000, fw)?;

        // Download the NVRAM to the 43xx device.
        defmt::info!("CYW43: nvram firmware: {}", nvram.len());
        let nvram_len = nvram.len();
        self.download_resource((CYW43_RAM_SIZE - 4 - nvram_len) as u32, nvram)?;

        let sz = ((!((nvram_len / 4) as u32) & 0xffff) << 16) | ((nvram_len / 4) as u32);

        self.bus.write_backplane(CYW43_RAM_SIZE as u32 - 4, sz, 4)?;

        self.reset_device_core(CoreId::WlanArm, false)?;
        self.device_core_is_up(CoreId::WlanArm)?;

        // wait until HT clock is available; takes about 29ms
        let mut ht_ready = false;
        for _ in 0..1000 {
            let reg = self
                .bus
                .read_reg::<u8>(Func::Backplane, SDIO_CHIP_CLOCK_CSR)?;

            if reg & SBSDIO_HT_AVAIL != 0 {
                defmt::info!("CYW43: HT ready");
                ht_ready = true;
                break;
            }

            delay_ms(1);
        }
        if !ht_ready {
            return Err(DevError::Io);
        }

        // interrupt mask
        self.bus
            .write_backplane(SDIO_INT_HOST_MASK, I_HMB_SW_MASK, 4)?;

        // Lower F2 Watermark to avoid DMA Hang in F2 when SD Clock is stopped.
        self.bus
            .write_reg::<u8>(Func::Backplane, SDIO_FUNCTION2_WATERMARK, SPI_F2_WATERMARK)?;

        Ok(())
    }

    pub(crate) fn clm_load(&mut self, clm: &[u8]) -> Result<(), DevError> {
        defmt::info!("CYW43: clm_load start size {}", clm.len());

        let payload_offset = SDPCM_HEADER_LEN + 16;
        let mut off = 0;

        while off < clm.len() {
            let mut len = core::cmp::min(CLM_CHUNK_LEN, clm.len() - off);

            let mut flag = DLOAD_HANDLER_VER;
            if off == 0 {
                flag |= DL_BEGIN;
            }

            if off + len >= clm.len() {
                flag |= DL_END;
                len = clm.len() - off;
            }
            {
                let buf = &mut self.spid_buf[payload_offset..];

                buf[..8].copy_from_slice(b"clmload\0");
                buf[8..10].copy_from_slice(&flag.to_le_bytes());
                buf[10..12].copy_from_slice(&DL_TYPE_CLM.to_le_bytes());
                buf[12..16].copy_from_slice(&(len as u32).to_le_bytes());
                buf[16..20].copy_from_slice(&0u32.to_le_bytes());
                buf[20..20 + len].copy_from_slice(&clm[off..off + len]);

                let ioctl_len = (20 + len + 0b111) & !0b111;

                self.do_ioctl(
                    SdpcmOp::Set,
                    WlcCmd::SetVar,
                    payload_offset,
                    ioctl_len,
                    Interface::STA,
                )?;

                off += len;
            }
            {
                let payload_len = 19;
                let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

                buf.fill(0);
                buf[0..15].copy_from_slice(b"clmload_status\0");
                self.do_ioctl(
                    SdpcmOp::Get,
                    WlcCmd::GetVar,
                    payload_offset,
                    payload_len,
                    Interface::STA,
                )?;

                let buf = &self.spid_buf[payload_offset..payload_offset + 4];
                let status = u32::from_le_bytes(buf.try_into().unwrap());
                if status != 0 {
                    defmt::warn!("clm_load failed");
                    return Err(DevError::InvalidArg);
                }
            }
        }

        defmt::info!("CYW43: clm_load done");

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn print_clm_version(&mut self) -> Result<(), DevError> {
        let payload_offset: usize = SDPCM_HEADER_LEN + 16;
        let payload_len: usize = 128;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf.fill(0);
            buf[..7].copy_from_slice(b"clmver\0");
        }

        self.do_ioctl(
            SdpcmOp::Get,
            WlcCmd::GetVar,
            payload_offset,
            payload_len,
            Interface::STA,
        )?;

        {
            let buf = &self.spid_buf[payload_offset..payload_offset + payload_len];

            if let Some(end) = buf.iter().position(|&b| b == 0) {
                if let Ok(s) = core::str::from_utf8(&buf[..end]) {
                    defmt::info!("clmver: {}", s);
                }
            }
        }

        Ok(())
    }
}
