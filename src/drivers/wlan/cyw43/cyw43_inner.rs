use rp235x_pac as pac;

use crate::{
    drivers::{
        delay_ms, gpio,
        wlan::cyw43::{cyw43_ioctl::Interface, firmware},
    },
    sys::device_driver::DevError,
};

use super::{
    Cyw43Inner,
    cyw43_bus::{CoreId, Cyw43Bus, Func},
    cyw43_regs::*,
    pio_spi,
};

const CYW43_BUS_MAX_BLOCK_SIZE: usize = 64;
#[allow(dead_code)]
const CYW43_BACKPLANE_READ_PAD_LEN_BYTES: usize = 16;

const CYW43_RAM_SIZE: usize = 512 * 1024;

impl Cyw43Inner {
    pub(crate) const fn new(
        gpio: &'static dyn gpio::interface::Gpio,
        wl_clk: usize,
        wl_dio: usize,
        wl_cs: usize,
        wl_on: usize,
    ) -> Self {
        let pio_spi = pio_spi::PioSpi::new(gpio, wl_clk, wl_dio, wl_cs);
        Self {
            gpio,

            wl_dio: gpio::Pin(wl_dio),
            wl_cs: gpio::Pin(wl_cs),
            wl_on: gpio::Pin(wl_on),
            bus: Cyw43Bus::new(pio_spi),

            sdpcm_seq: 0,
            packet_tx_seq: 0,
            last_bus_data_credit: 1,
            wlan_flow_control: 0,
            requested_ioctl_id: 0,
            had_successful_packet: false,
            spid_buf: [0; 2048],
            bus_is_up: false,
        }
    }

    pub(crate) fn init(&self) -> Result<(), DevError> {
        self.ll_init()?;

        Ok(())
    }

    pub(crate) fn init_hw(
        &mut self,
        pio0: pac::PIO0,
        resets: &mut pac::RESETS,
    ) -> Result<(), DevError> {
        // Init PIO SPI pins/program while CYW43 is still off.
        self.bus.init_hw(pio0, resets)?;

        Ok(())
    }

    fn gpio_config(&self) {
        // WL_ON
        self.gpio.pin_config(
            self.wl_on.0,
            gpio::Function::SIO,
            gpio::Pull::Up,
            Some(gpio::Direction::Output),
        );

        // WL_DIO
        self.gpio.pin_config(
            self.wl_dio.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Output),
        );
        self.gpio.set_level(&self.wl_dio, gpio::Level::Low);

        // WL_CS
        self.gpio.pin_config(
            self.wl_cs.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Output),
        );
        self.gpio.set_level(&self.wl_cs, gpio::Level::High);
    }

    fn spi_reset(&self) -> Result<(), DevError> {
        // Set WL_ON low
        self.gpio.set_level(&self.wl_on, gpio::Level::Low);
        delay_ms(20);

        // Set WL_ON high
        self.gpio.set_level(&self.wl_on, gpio::Level::High);
        delay_ms(250);

        // Set IRQ (WL_DIO) high
        self.gpio.pin_config(
            self.wl_dio.0,
            gpio::Function::SIO,
            gpio::Pull::None,
            Some(gpio::Direction::Input),
        );

        Ok(())
    }

    fn ll_init(&self) -> Result<(), DevError> {
        Ok(())
    }

    fn align_up(value: usize, align: usize) -> usize {
        (value + align - 1) & !(align - 1)
    }

    pub(crate) fn ensure_up(&mut self) -> Result<(), DevError> {
        if self.bus_is_up {
            return Ok(());
        }

        // Reset and power up the WL chip
        self.gpio.set_level(&self.wl_on, gpio::Level::Low);
        delay_ms(20);
        self.gpio.set_level(&self.wl_on, gpio::Level::High);
        delay_ms(50);

        self.gpio_config();

        self.spi_reset()?;

        self.bus.gpio_setup()?;

        self.bus.init()?;

        self.download_firmware(
            &firmware::CYW43_FW,
            firmware::CYW43_FW_LEN,
            &firmware::WIFI_NVRAM,
            firmware::WIFI_NVRAM_LEN,
        )?;

        self.bus.f2_ready()?;

        self.bus.bus_sleep(false)?;

        self.bus.clear_sdio_pull_up()?;

        self.bus.clear_data_unavailable()?;

        let clm_offset = Self::align_up(firmware::CYW43_FW_LEN, 512);

        self.clm_load(&firmware::CYW43_FW[clm_offset..])?;

        self.write_iovar_u32s("bus:txglom", &[0], Interface::STA)?;
        self.write_iovar_u32s("apsta", &[1], Interface::STA)?;

        self.set_mac()?;

        self.bus_is_up = true;

        Ok(())
    }

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

        defmt::info!("core is up");
        Ok(())
    }

    fn check_valid_chipset_firmware(&self, fw: &[u8], fw_size: usize) -> Result<(), DevError> {
        let fw_end_len = 800;

        if fw_size < fw_end_len || fw_size > fw.len() {
            return Err(DevError::InvalidArg);
        }

        let b = &fw[fw_size - fw_end_len..fw_size];

        let fw_end = fw_end_len - 16; // skip DVID trailer

        let trail_len = u16::from_le_bytes([b[fw_end - 2], b[fw_end - 1]]) as usize;

        if trail_len < 500 && b[fw_end - 3] == 0 {
            for i in 80..trail_len {
                let pos = fw_end - 3 - i;

                if pos + 9 <= b.len() && &b[pos..pos + 9] == b"Version: " {
                    defmt::info!("valid firmware found");
                    return Ok(());
                }
            }
        }

        Err(DevError::Io)
    }

    fn download_resource(&mut self, addr: u32, data: &[u8]) -> Result<(), DevError> {
        if data.len() & 0b11 != 0 {
            return Err(DevError::InvalidArg);
        }

        for offset in (0..data.len()).step_by(CYW43_BUS_MAX_BLOCK_SIZE) {
            let end = core::cmp::min(offset + CYW43_BUS_MAX_BLOCK_SIZE, data.len());
            let chunk = &data[offset..end];

            let dest_addr = addr + offset as u32;

            if ((dest_addr & BACKPLANE_ADDR_MASK) as usize + chunk.len())
                > (BACKPLANE_ADDR_MASK as usize + 1)
            {
                return Err(DevError::InvalidArg);
            }

            self.bus.set_backplane_window(dest_addr)?;

            let mut local_addr = dest_addr & BACKPLANE_ADDR_MASK;
            local_addr |= SBSDIO_SB_ACCESS_2_4B_FLAG;

            self.bus.write_bytes(Func::BackPlane, local_addr, chunk)?;
        }

        Ok(())
    }

    fn download_firmware(
        &mut self,
        fw: &[u8],
        fw_size: usize,
        nvram: &[u8],
        _nvram_size: usize,
    ) -> Result<(), DevError> {
        defmt::info!("downloading firmware");

        self.disable_device_core(CoreId::WlanArm, false)?;
        self.disable_device_core(CoreId::Socram, false)?;
        self.reset_device_core(CoreId::Socram, false)?;

        // this is 4343x specific stuff: Disable remap for SRAM_3
        self.bus.write_backplane(SOCSRAM_BANKX_INDEX, 0x03, 4)?;
        self.bus.write_backplane(SOCSRAM_BANKX_PDA, 0x00, 4)?;

        // Check that valid chipset firmware exists at the given source address.
        self.check_valid_chipset_firmware(fw, fw_size)?;

        // Download the main WiFi firmware blob to the 43xx device.
        defmt::info!("main firmware: {}", fw.len());
        self.download_resource(0x0000_0000, fw)?;

        // Download the NVRAM to the 43xx device.
        defmt::info!("nvram firmware: {}", nvram.len());
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
                .read_reg::<u8>(Func::BackPlane, SDIO_CHIP_CLOCK_CSR)?;

            if reg & SBSDIO_HT_AVAIL != 0 {
                defmt::info!("HT ready");
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
            .write_reg::<u8>(Func::BackPlane, SDIO_FUNCTION2_WATERMARK, SPI_F2_WATERMARK)?;

        Ok(())
    }

    fn clm_load(&mut self, clm: &[u8]) -> Result<(), DevError> {
        defmt::info!("clm_load start size {}", clm.len());

        defmt::info!("clm_load done");

        Ok(())
    }

    fn set_mac(&mut self) -> Result<(), DevError> {
        Ok(())
    }
}
