use super::{Cyw43Inner, cyw43_sdpcm::SDPCM_HEADER_LEN};
use crate::{
    drivers::{
        delay_ns,
        wlan::cyw43::{
            cyw43_bus::Cyw43Bus,
            cyw43_regs::*,
            cyw43_sdpcm::{SdpcmOp, SdpcmPacket, WlcCmd},
        },
    },
    sys::device_driver::DevError,
};

const CYW43_WL_GPIO_COUNT: usize = 3;
const CYW43_IOCTL_TIMEOUT_US: u64 = 500000;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum Interface {
    STA,
    AP,
    P2P,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IoctlHeader {
    pub cmd: u32,
    pub len: u32,
    pub flags: u32,
    pub status: u32,
}

pub(crate) const IOCTL_HEADER_LEN: usize = core::mem::size_of::<IoctlHeader>();

pub(crate) const CDCF_IOC_ID_MASK: u32 = 0xffff_0000;
pub(crate) const CDCF_IOC_ID_SHIFT: u32 = 16;
pub(crate) const CDCF_IOC_IF_SHIFT: u32 = 12;

impl Cyw43Inner {
    pub(crate) fn gpio_set(&mut self, pin: usize, level: bool) -> Result<(), DevError> {
        if pin >= CYW43_WL_GPIO_COUNT {
            return Err(DevError::InvalidArg);
        }
        defmt::info!("gpio_set {} {}", pin, level);

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
        defmt::info!("send_ioctl");
        if SDPCM_HEADER_LEN + IOCTL_HEADER_LEN + payload_len > self.spid_buf.len() {
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

    pub(crate) fn do_ioctl(
        &mut self,
        kind: SdpcmOp,
        cmd: WlcCmd,
        payload_offset: usize,
        payload_len: usize,
        iface: Interface,
    ) -> Result<(), DevError> {
        self.send_ioctl(kind, cmd, payload_offset, payload_len, iface)?;

        let start = Cyw43Bus::tick_us();

        while Cyw43Bus::tick_us().wrapping_sub(start) < CYW43_IOCTL_TIMEOUT_US {
            match self.sdpcm_poll_device()? {
                SdpcmPacket::Control {
                    offset: res_offset,
                    len: res_len,
                } => {
                    let copy_len = core::cmp::min(payload_len, res_len);
                    self.spid_buf
                        .copy_within(res_offset..res_offset + copy_len, payload_offset);
                    return Ok(());
                }
                SdpcmPacket::AsyncEvent { offset, len } => {
                    // ToDo: parse async event
                    let _ = (offset, len);
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

        Err(DevError::Timeout)
    }

    pub(crate) fn write_iovar_u32s(
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

        self.do_ioctl(
            SdpcmOp::Set,
            WlcCmd::SetVar,
            payload_offset,
            payload_len,
            iface,
        )
    }
}
