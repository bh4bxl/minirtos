use crate::{drivers::delay_ns, sys::device_driver::DevError};

use super::{
    Cyw43Inner, PendingBuf, PendingIoctlResp,
    cyw43_regs::*,
    cyw43_sdpcm::SDPCM_HEADER_LEN,
    cyw43_sdpcm::{SdpcmOp, SdpcmPacket, WlcCmd},
};

const CYW43_WL_GPIO_COUNT: usize = 3;
const CYW43_IOCTL_TIMEOUT_US: u64 = 500000;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum Interface {
    STA = 0,
    AP = 1,
    P2P = 2,
}

impl Interface {
    pub(super) fn from_u8(v: u8) -> Self {
        match v {
            0 => Interface::STA,
            1 => Interface::AP,
            2 => Interface::P2P,
            _ => {
                defmt::warn!("CYW43: unknown interface {}", v);
                Interface::STA
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(super) struct IoctlHeader {
    pub cmd: u32,
    pub len: u32,
    pub flags: u32,
    pub status: u32,
}

pub(super) const IOCTL_HEADER_LEN: usize = core::mem::size_of::<IoctlHeader>();

pub(super) const CDCF_IOC_ID_MASK: u32 = 0xffff_0000;
pub(super) const CDCF_IOC_ID_SHIFT: u32 = 16;
pub(super) const CDCF_IOC_IF_SHIFT: u32 = 12;

impl Cyw43Inner {
    pub(super) fn gpio_set(&mut self, pin: usize, level: bool) -> Result<(), DevError> {
        if pin >= CYW43_WL_GPIO_COUNT {
            defmt::warn!("invalid gpio {}", pin);
            return Err(DevError::InvalidArg);
        }
        defmt::info!("CYW43: gpio_set {} {}", pin, level);

        self.write_iovar_u32s(
            "gpioout",
            &[1u32 << pin, if level { 1 << pin } else { 0 }],
            Interface::STA,
        )?;

        Ok(())
    }

    fn send_ioctl(
        &mut self,
        kind: SdpcmOp,
        cmd: WlcCmd,
        payload_offset: usize,
        payload_len: usize,
        iface: Interface,
    ) -> Result<(), DevError> {
        if SDPCM_HEADER_LEN + IOCTL_HEADER_LEN + payload_len > self.spid_buf.len() {
            defmt::warn!("payload_len {} too large", payload_len);
            return Err(DevError::InvalidArg);
        }

        debug_assert_eq!(payload_offset, SDPCM_HEADER_LEN + IOCTL_HEADER_LEN);

        self.requested_ioctl_id = self.requested_ioctl_id.wrapping_add(1);

        let flags = (((self.requested_ioctl_id as u32) << CDCF_IOC_ID_SHIFT) & CDCF_IOC_ID_MASK)
            | (kind as u32)
            | ((iface as u32) << CDCF_IOC_IF_SHIFT);

        let hdr = &mut self.spid_buf[SDPCM_HEADER_LEN..SDPCM_HEADER_LEN + IOCTL_HEADER_LEN];

        hdr[0..4].copy_from_slice(&(cmd as u32).to_le_bytes());
        hdr[4..8].copy_from_slice(&((payload_len as u32) & 0xffff).to_le_bytes());
        hdr[8..12].copy_from_slice(&flags.to_le_bytes());
        hdr[12..16].copy_from_slice(&0u32.to_le_bytes());

        self.sdpcm_send_common(CONTROL_HEADER, IOCTL_HEADER_LEN + payload_len)
    }

    fn finish_ioctl_resp(
        &mut self,
        cmd: WlcCmd,
        payload_offset: usize,
        payload_len: usize,
        resp: PendingIoctlResp,
    ) -> Result<(), DevError> {
        if resp.status != 0 {
            defmt::warn!("CYW43: cmd={} failed status={}", cmd as u32, resp.status);
            return Err(DevError::Io);
        }

        let copy_len = core::cmp::min(payload_len, resp.buf.len);

        self.spid_buf
            .copy_within(resp.buf.offset..resp.buf.offset + copy_len, payload_offset);

        Ok(())
    }

    pub(super) fn do_ioctl(
        &mut self,
        kind: SdpcmOp,
        cmd: WlcCmd,
        payload_offset: usize,
        payload_len: usize,
        iface: Interface,
    ) -> Result<(), DevError> {
        self.send_ioctl(kind, cmd, payload_offset, payload_len, iface)?;

        if let Some(resp) = self.pending_ioctl_resp.take() {
            return self.finish_ioctl_resp(cmd, payload_offset, payload_len, resp);
        }

        let start = super::ticks_us();

        while super::ticks_us().wrapping_sub(start) < CYW43_IOCTL_TIMEOUT_US {
            match self.sdpcm_poll_device()? {
                SdpcmPacket::Control {
                    offset,
                    len,
                    status,
                } => {
                    if status != 0 {
                        defmt::warn!("CYW43: cmd={} failed status={}", cmd as u32, status);
                        return Err(DevError::Io);
                    }

                    return self.finish_ioctl_resp(
                        cmd,
                        payload_offset,
                        payload_len,
                        PendingIoctlResp {
                            buf: PendingBuf { offset, len },
                            status,
                        },
                    );
                }
                SdpcmPacket::AsyncEvent { offset, len } => {
                    self.handle_async_event(offset, len)?;
                }
                SdpcmPacket::Data {
                    offset,
                    len,
                    interface,
                } => {
                    // TODO: process ethernet frame
                    let _ = (offset, len, interface);
                }
                SdpcmPacket::None => {
                    // no packet
                }
                SdpcmPacket::Unexpected(kind) => {
                    defmt::warn!("do_ioctl: unexpected packet {}", kind);
                }
            }

            delay_ns(10000);
        }

        defmt::warn!("CYW43:do_ioctl: timeout");

        Err(DevError::Timeout)
    }

    pub(super) fn write_iovar_u32s(
        &mut self,
        var: &str,
        vals: &[u32],
        iface: Interface,
    ) -> Result<(), DevError> {
        let payload_offset = SDPCM_HEADER_LEN + 16;
        let payload_len = var.len() + 1 + vals.len() * 4;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf[..var.len()].copy_from_slice(var.as_bytes());
            buf[var.len()] = 0;

            let mut off = var.len() + 1;
            for &val in vals {
                buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
                off += 4;
            }
        }

        if let Err(x) = self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetVar,
            payload_offset,
            payload_len,
            iface,
        ) {
            defmt::warn!("write_iovar_u32s {} failed", var);
            return Err(x);
        }
        Ok(())
    }

    pub(super) fn write_iovar_n(
        &mut self,
        var: &str,
        data: &[u8],
        iface: Interface,
    ) -> Result<(), DevError> {
        let payload_offset = SDPCM_HEADER_LEN + 16;
        let payload_len = var.len() + 1 + data.len();

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf[..var.len()].copy_from_slice(var.as_bytes());
            buf[var.len()] = 0;

            let data_off = var.len() + 1;

            buf[data_off..data_off + data.len()].copy_from_slice(data);
        }

        if let Err(x) = self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetVar,
            payload_offset,
            payload_len,
            iface,
        ) {
            defmt::warn!("write_iovar_n {} failed", var);
            return Err(x);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(super) fn get_ioctl_u32(&mut self, cmd: WlcCmd, iface: Interface) -> Result<u32, DevError> {
        let payload_offset: usize = SDPCM_HEADER_LEN + 16;
        let payload_len: usize = 4;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];
            buf.fill(0);
        }

        self.do_ioctl(SdpcmOp::Get, cmd, payload_offset, payload_len, iface)?;

        let val = {
            let buf = &self.spid_buf[payload_offset..payload_offset + payload_len];
            u32::from_le_bytes(buf.try_into().unwrap())
        };

        Ok(val)
    }

    pub(super) fn set_ioctl_u32(
        &mut self,
        cmd: WlcCmd,
        val: u32,
        iface: Interface,
    ) -> Result<(), DevError> {
        let payload_offset = SDPCM_HEADER_LEN + 16;
        let payload_len = 4usize;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf[0..4].copy_from_slice(&val.to_le_bytes());
        }

        self.do_ioctl(SdpcmOp::Set, cmd, payload_offset, payload_len, iface)
    }

    fn clear_event(buf: &mut [u8], event: usize) {
        let base = 18 + 4;
        buf[base + event / 8] &= !(1u8 << (event % 8));
    }

    pub(super) fn set_event_msgs(&mut self) -> Result<(), DevError> {
        let payload_offset: usize = SDPCM_HEADER_LEN + 16;
        let event_mask_len: usize = 19;
        let payload_len: usize = 18 + 4 + event_mask_len;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf.fill(0);

            // iovar name, no trailing '\0' here. SDK uses exactly 18 bytes.
            buf[0..18].copy_from_slice(b"bsscfg:event_msgs\0");

            // bsscfg index = 0
            buf[18..22].copy_from_slice(&0u32.to_le_bytes());

            // Enable all async events first.
            buf[22..22 + event_mask_len].fill(0xff);

            // Then clear noisy / unwanted event bits.
            Self::clear_event(buf, 19);
            Self::clear_event(buf, 20);
            Self::clear_event(buf, 40);
            Self::clear_event(buf, 44);
            Self::clear_event(buf, 54);
            Self::clear_event(buf, 71);
        }

        self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetVar,
            payload_offset,
            payload_len,
            Interface::STA,
        )
    }

    pub(super) fn wlc_up(&mut self) -> Result<(), DevError> {
        self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::Up,
            SDPCM_HEADER_LEN + IOCTL_HEADER_LEN,
            0,
            Interface::STA,
        )
    }

    pub(crate) fn set_ssid(&mut self, ssid: &str) -> Result<(), DevError> {
        let ssid_len = ssid.len();

        if ssid_len > 32 {
            return Err(DevError::InvalidArg);
        }

        let payload_offset = SDPCM_HEADER_LEN + IOCTL_HEADER_LEN;
        let payload_len = 36;

        {
            let buf = &mut self.spid_buf[payload_offset..payload_offset + payload_len];

            buf.fill(0);

            buf[0..4].copy_from_slice(&(ssid_len as u32).to_le_bytes());
            buf[4..4 + ssid_len].copy_from_slice(ssid.as_bytes());
        }

        self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetSsid,
            payload_offset,
            payload_len,
            Interface::STA,
        )
    }
}
