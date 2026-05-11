use crate::{drivers::delay_us, net::ScanResult, sys::device_driver::DevError};

use super::{
    Cyw43Inner,
    cyw43_bus::Func,
    cyw43_consts::*,
    cyw43_ioctl::{IOCTL_HEADER_LEN, IoctlHeader},
    cyw43_regs::*,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SdpcmHeader {
    pub size: u16,
    pub size_com: u16,

    pub sequence: u8,
    pub channel_and_flags: u8,

    pub next_length: u8,
    pub header_length: u8,

    pub wireless_flow_control: u8,
    pub bus_data_credit: u8,

    pub reserved: [u8; 2],
}

pub(super) const SDPCM_HEADER_LEN: usize = core::mem::size_of::<SdpcmHeader>();

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(super) enum SdpcmOp {
    Get = 0,
    Set = 2,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum WlcCmd {
    Up = 2,

    SetInfra = 20,
    SetAuth = 22,

    GetBssid = 23,
    GetSsid = 25,
    SetSsid = 26,

    SetChannel = 30,

    Disassoc = 52,

    GetAntDiv = 63,
    SetAntDiv = 64,

    SetDtimPrd = 78,

    GetPm = 85,
    SetPm = 86,

    SetGMode = 110,

    SetWsec = 134,

    SetBand = 142,

    GetAssocList = 159,

    SetWpaAuth = 165,

    GetVar = 262,
    SetVar = 263,

    SetWsecPmk = 268,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum SdpcmPacket {
    None,
    Control {
        offset: usize,
        len: usize,
        status: i32,
    },
    AsyncEvent {
        offset: usize,
        len: usize,
    },
    Data {
        offset: usize,
        len: usize,
        interface: u8,
    },
    Unexpected(u8),
}

#[repr(C, align(4))]
#[derive(Clone, Copy, Default)]
struct Cyw43EvScanResult {
    _0: [u32; 5],
    bssid: [u8; 6],
    _1: [u16; 2],
    ssid_len: u8,
    ssid: [u8; 32],
    _2: [u32; 5],
    channel: u16,
    _3: u16,
    auth_mode: u8,
    rssi: i16,
}

#[repr(C, align(4))]
#[derive(Clone, Copy, Default)]
struct Cyw43AsyncEvent {
    _0: u16,
    flags: u16,
    event_type: u32,
    status: u32,
    reason: u32,
    _1: [u8; 30],
    interface: u8,
    _2: u8,
    scan_result: Cyw43EvScanResult,
}

pub(super) const PAYLOAD_MTU: usize = 1500;
pub(super) const LINK_HEADER: usize = 30;
pub(super) const ETHERNET_SIZE: usize = 14;
pub(super) const LINK_MTU: usize = PAYLOAD_MTU + LINK_HEADER + ETHERNET_SIZE;
pub(super) const GSPI_PACKET_OVERHEAD: usize = 8;

const SDPCM_SEND_TIMEOUT_US: u64 = 1_000_000;

impl Cyw43Inner {
    pub(super) fn sdpcm_send_common(
        &mut self,
        kind: u8,
        payload_len: usize,
    ) -> Result<(), DevError> {
        if kind != CONTROL_HEADER && kind != DATA_HEADER {
            defmt::warn!("CYW43: invalid sdpcm kind {}", kind);
            return Err(DevError::InvalidArg);
        }

        self.bus.bus_sleep(false)?;

        // Wait until firmware gives us TX credit.
        if self.wlan_flow_control != 0 || self.last_bus_data_credit == self.packet_tx_seq {
            let start_us = super::ticks_us();

            loop {
                match self.sdpcm_poll_device()? {
                    SdpcmPacket::AsyncEvent { offset, len } => {
                        // TODO: process async event
                        self.handle_async_event(offset, len)?;
                    }

                    SdpcmPacket::Data {
                        offset,
                        len,
                        interface,
                    } => {
                        // TODO: do not process here yet, avoid reentrancy
                        defmt::debug!(
                            "CYW43: data packet while waiting credit offset={} len={} iface={}",
                            offset,
                            len,
                            interface
                        );
                    }

                    SdpcmPacket::Control { .. } => {
                        // Usually ignored here. do_ioctl() will wait for its own response.
                        defmt::warn!(
                            "CYW43: sdpcm_send_common got control packet while waiting credit"
                        );
                        return Err(DevError::Busy);
                    }

                    SdpcmPacket::None => {}

                    SdpcmPacket::Unexpected(ch) => {
                        defmt::warn!("sdpcm stall: unexpected packet {}", ch);
                    }
                }

                if self.wlan_flow_control == 0 && self.last_bus_data_credit != self.packet_tx_seq {
                    break;
                }

                if super::ticks_us().wrapping_sub(start_us) > SDPCM_SEND_TIMEOUT_US {
                    defmt::warn!(
                        "CYW43: sdpcm stall timeout flow={} tx_seq={} credit={}",
                        self.wlan_flow_control,
                        self.packet_tx_seq,
                        self.last_bus_data_credit
                    );
                    return Err(DevError::Timeout);
                }

                delay_us(100);
            }
        }

        let size = SDPCM_HEADER_LEN + payload_len;

        if super::align_up(size, 0b100) > self.spid_buf.len() {
            defmt::warn!("CYW43: payload_len {} too large", payload_len);
            return Err(DevError::InvalidArg);
        }

        // SDPCM header at spid_buf[0..SDPCM_HEADER_LEN]
        let size_u16 = size as u16;
        let size_com = !size_u16;

        self.spid_buf[0..2].copy_from_slice(&size_u16.to_le_bytes());
        self.spid_buf[2..4].copy_from_slice(&size_com.to_le_bytes());

        self.spid_buf[4] = self.packet_tx_seq;
        self.spid_buf[5] = kind;

        self.spid_buf[6] = 0; // next_length

        self.spid_buf[7] = if kind == DATA_HEADER {
            (SDPCM_HEADER_LEN + 2) as u8
        } else {
            SDPCM_HEADER_LEN as u8
        };

        self.spid_buf[8] = 0; // wireless_flow_control
        self.spid_buf[9] = 0; // bus_data_credit
        self.spid_buf[10] = 0; // reserved[0]
        self.spid_buf[11] = 0; // reserved[1]

        self.packet_tx_seq = self.packet_tx_seq.wrapping_add(1);

        let write_len = super::align_up(size, 0b100);

        self.bus
            .write_bytes(Func::Wlan, 0, &self.spid_buf[..write_len])?;

        Ok(())
    }

    pub(super) fn sdpcm_process_rx_packet(
        &mut self,
        _packet_len: usize,
    ) -> Result<SdpcmPacket, DevError> {
        let header = unsafe { &*(self.spid_buf.as_ptr() as *const SdpcmHeader) };

        let size = u16::from_le(header.size) as usize;
        let size_com = u16::from_le(header.size_com);

        if size_com != !(size as u16) {
            defmt::warn!("CYW43: invalid sdpcm header");
            return Ok(SdpcmPacket::None);
        }

        if size < SDPCM_HEADER_LEN {
            defmt::warn!("CYW43: packet too small");
            return Ok(SdpcmPacket::None);
        }

        self.wlan_flow_control = header.wireless_flow_control;

        let channel = header.channel_and_flags & 0x0f;

        if channel < 3 {
            let credit = header
                .bus_data_credit
                .wrapping_sub(self.last_bus_data_credit);
            if credit <= 20 {
                self.last_bus_data_credit = header.bus_data_credit;
            }
        }

        match channel {
            CONTROL_HEADER => {
                if size < SDPCM_HEADER_LEN + IOCTL_HEADER_LEN {
                    defmt::warn!("CYW43: control packet too small");
                    return Ok(SdpcmPacket::None);
                }

                let ioctl_offset = header.header_length as usize;

                let ioctl =
                    unsafe { &*(self.spid_buf[ioctl_offset..].as_ptr() as *const IoctlHeader) };

                let id = (u32::from_le(ioctl.flags) & CDCF_IOC_ID_MASK) >> CDCF_IOC_ID_SHIFT;

                if id != self.requested_ioctl_id as u32 {
                    defmt::warn!(
                        "CYW43: wrong ioctl id {} != {}",
                        id,
                        self.requested_ioctl_id
                    );

                    return Ok(SdpcmPacket::None);
                }

                let payload_offset = ioctl_offset + IOCTL_HEADER_LEN;

                let payload_len = size - payload_offset;

                return Ok(SdpcmPacket::Control {
                    offset: payload_offset,
                    len: payload_len,
                    status: u32::from_le(ioctl.status) as i32,
                });
            }

            DATA_HEADER => {
                defmt::warn!("CYW43: data packet ignored");
                Ok(SdpcmPacket::None)
            }

            ASYNCEVENT_HEADER => {
                let payload_offset = header.header_length as usize;
                let payload_len = size - payload_offset;
                defmt::debug!(
                    "CYW43: [EVENT] async event offset={} len={}",
                    payload_offset,
                    payload_len,
                );
                Ok(SdpcmPacket::AsyncEvent {
                    offset: payload_offset,
                    len: payload_len,
                })
            }

            _ => {
                defmt::warn!("CYW43: unknown sdpcm channel {}", channel);
                Ok(SdpcmPacket::None)
            }
        }
    }

    pub(super) fn sdpcm_poll_device(&mut self) -> Result<SdpcmPacket, DevError> {
        self.bus.bus_sleep(false)?;

        if !self.had_successful_packet {
            let spi_int = self
                .bus
                .read_reg::<u16>(Func::Bus, SPI_INTERRUPT_REGISTER)?;

            if spi_int & BUS_OVERFLOW_UNDERFLOW != 0 {
                defmt::warn!("bus overflow/underflow: 0x{:04x}", spi_int);
            }

            if spi_int != 0 {
                self.bus
                    .write_reg::<u16>(Func::Bus, SPI_INTERRUPT_REGISTER, spi_int)?;
            }

            if spi_int & F2_PACKET_AVAILABLE == 0 {
                return Ok(SdpcmPacket::None);
            }
        }

        let mut status = 0xffff_ffff;

        for _ in 0..1000 {
            status = self.bus.read_reg::<u32>(Func::Bus, SPI_STATUS_REGISTER)?;
            if status != 0xffff_ffff {
                break;
            }
            delay_us(1);
        }

        if status == 0xffff_ffff {
            return Ok(SdpcmPacket::None);
        }

        if status & GSPI_PACKET_AVAILABLE as u32 == 0 {
            self.had_successful_packet = false;
            return Ok(SdpcmPacket::None);
        }

        let bytes_pending = ((status & STATUS_F2_PKT_LEN_MASK) >> STATUS_F2_PKT_LEN_SHIFT) as usize;

        if bytes_pending == 0
            || bytes_pending > LINK_MTU - GSPI_PACKET_OVERHEAD
            || (status & STATUS_UNDERFLOW) != 0
        {
            defmt::warn!("CYW43: invalid bytes_pending {}", bytes_pending);

            self.bus
                .write_reg::<u8>(Func::Backplane, SPI_FRAME_CONTROL, 1 << 0)?;

            self.had_successful_packet = false;
            return Ok(SdpcmPacket::None);
        }

        let len = bytes_pending;
        let xfer_len = 4 + ((len + 3) & !3);

        self.bus
            .read_bytes(Func::Wlan, 0, len, &mut self.spid_buf[..xfer_len])?;

        let size = u16::from_le_bytes([self.spid_buf[0], self.spid_buf[1]]);
        let size_com = u16::from_le_bytes([self.spid_buf[2], self.spid_buf[3]]);

        if size == 0 && size_com == 0 {
            self.had_successful_packet = false;
            return Ok(SdpcmPacket::None);
        }

        self.had_successful_packet = true;

        if size ^ size_com != 0xffff {
            defmt::warn!("CYW43: sdpcm hdr mismatch {:04x} ^ {:04x}", size, size_com);
            return Ok(SdpcmPacket::None);
        }

        self.sdpcm_process_rx_packet(bytes_pending)
    }

    pub(super) fn handle_async_event(&mut self, offset: usize, len: usize) -> Result<(), DevError> {
        let event_offset_in_sdpcm_playload: usize = 32;

        if len < event_offset_in_sdpcm_playload + 16 {
            defmt::warn!("CYW43: [EVENT] too short len={}", len);
            return Ok(());
        }

        let ev_offset = offset + event_offset_in_sdpcm_playload;
        let ev_len = len - event_offset_in_sdpcm_playload;
        let ev_size = core::mem::size_of::<Cyw43AsyncEvent>();
        let mut ev = Cyw43AsyncEvent::default();

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.spid_buf[ev_offset..].as_ptr(),
                &mut ev as *mut _ as *mut u8,
                core::cmp::min(ev_size, ev_len),
            );
        }

        let flags = u16::from_be(ev.flags);
        let event_type = u32::from_be(ev.event_type);
        let status = u32::from_be(ev.status);
        let reason = u32::from_be(ev.reason);

        defmt::debug!(
            "CYW43: [EVENT] flags=0x{:04x} type={} status={} reason={}",
            flags,
            event_type,
            status,
            reason,
        );

        match event_type {
            event::ESCAN_RESULT => {
                // CYW43_EV_ESCAN_RESULT
                if status == status::PARTIAL {
                    defmt::debug!("CYW43: [SCAN] partial result");

                    let mut res = ScanResult {
                        ssid: [0; 32],
                        ssid_len: core::cmp::min(ev.scan_result.ssid_len as usize, 32),
                        bssid: ev.scan_result.bssid,
                        channel: u16::from_le(ev.scan_result.channel) & 0xff,
                        rssi: i16::from_le(ev.scan_result.rssi),
                    };

                    res.ssid[..res.ssid_len].copy_from_slice(&ev.scan_result.ssid[..res.ssid_len]);

                    if let Ok(ssid_str) = core::str::from_utf8(&res.ssid) {
                        defmt::info!(
                            "CYW43: [SCAN] ssid={} channel={} rssi={} bssid={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            ssid_str,
                            res.channel,
                            i16::from_le(ev.scan_result.rssi),
                            res.bssid[0],
                            res.bssid[1],
                            res.bssid[2],
                            res.bssid[3],
                            res.bssid[4],
                            res.bssid[5],
                        );
                    } else {
                        defmt::info!(
                            "CYW43: [SCAN] ssid=<non-utf8> channel={} rssi={} bssid={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            res.channel,
                            res.rssi,
                            res.bssid[0],
                            res.bssid[1],
                            res.bssid[2],
                            res.bssid[3],
                            res.bssid[4],
                            res.bssid[5],
                        );
                    }

                    if !self
                        .scan_results
                        .iter()
                        .any(|r| r.bssid == res.bssid && r.channel == res.channel)
                    {
                        self.scan_results.push(res).ok();
                    }
                } else if status == status::SUCCESS {
                    self.scan_done = true;
                    self.scan_in_progress = false;
                    defmt::info!("CYW43: [SCAN] complete");
                } else {
                    defmt::warn!("CYW43: [SCAN] escan status={}", status);
                }
            }
            _ => {
                defmt::debug!("CYW43: [EVENT] unhandled type={}", event_type);
            }
        }

        Ok(())
    }

    pub(super) fn poll(&mut self) -> Result<(), DevError> {
        loop {
            match self.sdpcm_poll_device()? {
                SdpcmPacket::AsyncEvent { offset, len } => {
                    self.handle_async_event(offset, len)?;
                }

                SdpcmPacket::Data {
                    offset,
                    len,
                    interface,
                } => {
                    defmt::info!(
                        "CYW43: [DATA] offset={} len={} iface={}",
                        offset,
                        len,
                        interface
                    );
                }

                SdpcmPacket::Control { .. } => {
                    defmt::warn!("CYW43: poll got unexpected control packet");
                }

                SdpcmPacket::None => {
                    break;
                }

                SdpcmPacket::Unexpected(ch) => {
                    defmt::warn!("CYW43: [RX] unexpected channel {}", ch);
                }
            }
        }
        Ok(())
    }
}
