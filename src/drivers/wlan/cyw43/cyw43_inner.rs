use rp235x_pac as pac;

use crate::{
    drivers::{
        delay_ms, delay_us, gpio,
        wlan::cyw43::{
            cyw43_fw,
            cyw43_ioctl::Interface,
            cyw43_sdpcm::{SDPCM_HEADER_LEN, SdpcmOp, WlcCmd},
        },
    },
    sys::device_driver::DevError,
};

use super::{Cyw43Inner, cyw43_bus::Cyw43Bus, pio_spi};

#[repr(C)]
struct WifiScanOptions {
    version: u32,
    action: u16,
    reserved: u16,
    ssid_len: u32,
    ssid: [u8; 32],
    bssid: [u8; 6],
    bss_type: i8,
    scan_type: i8,
    nprobes: i32,
    active_time: i32,
    passive_time: i32,
    home_time: i32,
    channel_num: i32,
    channel_list: [u16; 1],
}

const _: () = assert!(core::mem::size_of::<WifiScanOptions>() == 76);

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
            startup_t0: 0,

            bus_is_up: false,
        }
    }

    pub(crate) fn init(&self) -> Result<(), DevError> {
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

    pub(crate) fn wifi_on(&mut self, country: u32) -> Result<(), DevError> {
        self.ensure_up()?;

        self.wifi_init_sta(country)?;

        Ok(())
    }

    pub(crate) fn ensure_up(&mut self) -> Result<(), DevError> {
        if self.bus_is_up {
            return Ok(());
        }

        self.ll_bus_init()?;

        self.startup_t0 = super::ticks_us();

        Ok(())
    }

    fn ll_bus_init(&mut self) -> Result<(), DevError> {
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
            &cyw43_fw::CYW43_FW,
            cyw43_fw::CYW43_FW_LEN,
            &cyw43_fw::WIFI_NVRAM,
            cyw43_fw::WIFI_NVRAM_LEN,
        )?;

        self.bus.f2_ready()?;

        self.bus.bus_sleep(false)?;

        self.bus.clear_sdio_pull_up()?;

        self.bus.clear_data_unavailable()?;

        let clm_offset = super::align_up(cyw43_fw::CYW43_FW_LEN, 512);

        self.clm_load(&cyw43_fw::CYW43_FW[clm_offset..])?;

        self.write_iovar_u32s("bus:txglom", &[0], Interface::STA)?; // tx glomming off
        self.write_iovar_u32s("apsta", &[1], Interface::STA)?; // apsta on

        self.set_mac()?;

        self.bus_is_up = true;

        Ok(())
    }

    fn set_mac(&mut self) -> Result<(), DevError> {
        Ok(())
    }

    fn wifi_init_sta(&mut self, country: u32) -> Result<(), DevError> {
        self.set_country(country)?;

        // self.print_clm_version()?;

        self.set_ioctl_u32(WlcCmd::SetAntDiv, 0, Interface::STA)?;

        self.write_iovar_u32s("bus:txglom", &[0], Interface::STA)?;
        self.write_iovar_u32s("apsta", &[1], Interface::STA)?;
        self.write_iovar_u32s("ampdu_ba_wsize", &[8], Interface::STA)?;
        self.write_iovar_u32s("ampdu_mpdu", &[4], Interface::STA)?;
        self.write_iovar_u32s("ampdu_rx_factor", &[0], Interface::STA)?;

        let elapsed_us = super::ticks_us().wrapping_sub(self.startup_t0) as u32;
        if elapsed_us < 150_000 {
            delay_us(150_000 - elapsed_us);
        }

        // This delay is needed for the WLAN chip to do some processing, otherwise
        // SDIOIT/OOB WL_HOST_WAKE IRQs in bus-sleep mode do no work correctly.
        self.set_event_msgs()?;
        delay_ms(50);

        self.wlc_up()?;
        delay_ms(50);

        Ok(())
    }

    pub(crate) fn wifi_scan(&mut self) -> Result<(), DevError> {
        let payload_offset: usize = SDPCM_HEADER_LEN + 16;
        let name_len: usize = 6; // "escan\0"
        let opt_len: usize = 76;
        let payload_len: usize = name_len + opt_len;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf.fill(0);
            buf[0..6].copy_from_slice(b"escan\0");

            let p = &mut buf[6..6 + opt_len];

            p[0..4].copy_from_slice(&1u32.to_le_bytes()); // version
            p[4..6].copy_from_slice(&1u16.to_le_bytes()); // action = START
            p[6..8].copy_from_slice(&0u16.to_le_bytes()); // reserved
            p[8..12].copy_from_slice(&0u32.to_le_bytes()); // ssid_len = all

            p[12..44].fill(0); // ssid
            p[44..50].fill(0xff); // bssid

            p[50] = 2; // bss_type = ANY
            p[51] = 0; // scan_type = active

            p[52..56].copy_from_slice(&(-1i32).to_le_bytes()); // nprobes
            p[56..60].copy_from_slice(&(-1i32).to_le_bytes()); // active_time
            p[60..64].copy_from_slice(&(-1i32).to_le_bytes()); // passive_time
            p[64..68].copy_from_slice(&(-1i32).to_le_bytes()); // home_time

            p[68..72].copy_from_slice(&0i32.to_le_bytes()); // channel_num
            p[72..74].copy_from_slice(&0u16.to_le_bytes()); // channel_list[0]
            p[74..76].fill(0); // padding
        }

        self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetVar,
            payload_offset,
            payload_len,
            Interface::STA,
        )
    }
}
