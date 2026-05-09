use crate::{
    drivers::{
        delay_ms,
        wlan::cyw43::{
            cyw43_ioctl::Interface,
            cyw43_sdpcm::{SdpcmOp, WlcCmd},
        },
    },
    sys::device_driver::DevError,
};

use super::{Cyw43Inner, cyw43_sdpcm::SDPCM_HEADER_LEN};

const fn cyw43_country(a: u8, b: u8, rev: u32) -> u32 {
    (a as u32) | ((b as u32) << 8) | (rev << 16)
}

#[allow(dead_code)]
pub const CYW43_COUNTRY_WORLDWIDE: u32 = cyw43_country(b'X', b'X', 0);
pub const CYW43_COUNTRY_CANADA: u32 = cyw43_country(b'C', b'A', 0);
#[allow(dead_code)]
pub const CYW43_COUNTRY_CHINA: u32 = cyw43_country(b'C', b'N', 0);
#[allow(dead_code)]
pub const CYW43_COUNTRY_USA: u32 = cyw43_country(b'U', b'S', 0);

impl Cyw43Inner {
    pub(crate) fn set_country(&mut self, country: u32) -> Result<(), DevError> {
        let payload_offset = SDPCM_HEADER_LEN + 16;
        let payload_len = 20usize;

        let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

        buf[0..8].copy_from_slice(b"country\0");

        // country_abbrev
        buf[8..12].copy_from_slice(&(country & 0xffff).to_le_bytes());

        // revision
        let rev = if (country >> 16) == 0 {
            u32::MAX
        } else {
            country >> 16
        };
        buf[12..16].copy_from_slice(&rev.to_le_bytes());

        // ccode
        buf[16..20].copy_from_slice(&(country & 0xffff).to_le_bytes());

        self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetVar,
            payload_offset,
            payload_len,
            Interface::STA,
        )?;

        delay_ms(50);

        Ok(())
    }
}
