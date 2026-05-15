use crate::{
    drivers::delay_us,
    net::{ScanResult, WifiState, WlanPollResult},
    sys::device_driver::DevError,
};

use super::{
    Cyw43Inner, PendingBuf, PendingIoctlResp, cyw43_bus::Func, cyw43_consts::*,
    cyw43_ioctl::IOCTL_HEADER_LEN, cyw43_ioctl::Interface, cyw43_regs::*,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SdpcmBdcHeader {
    pub flags: u8,
    pub priority: u8,
    pub flags2: u8,
    pub data_offset: u8,
}

pub(super) const BDC_HEADER_LEN: usize = core::mem::size_of::<SdpcmBdcHeader>();

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
        interface: Interface,
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

impl super::WifiAuth {
    pub(super) fn as_u32(self) -> u32 {
        match self {
            Self::Open => 0,
            Self::WpaTkipPsk => 0x0020_0002,
            Self::Wpa2AesPsk => 0x0040_0004,
            Self::Wpa2MixedPsk => 0x0040_0006,
            Self::Wpa3SaeAesPsk => 0x0100_0004,
            Self::Wpa3Wpa2AesPsk => 0x0140_0004,
        }
    }
}

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
                        self.handle_async_event(offset, len)?;
                    }

                    SdpcmPacket::Data { .. } => {
                        // Do nothing
                    }

                    SdpcmPacket::Control {
                        offset,
                        len,
                        status,
                    } => {
                        self.handle_control_packet(offset, len, status);

                        continue;
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

        let header = unsafe { &mut *(self.spid_buf.as_mut_ptr() as *mut SdpcmHeader) };

        header.size = (size as u16).to_le();
        header.size_com = (!(size as u16)).to_le();
        header.sequence = self.packet_tx_seq;
        header.channel_and_flags = kind;
        header.next_length = 0;
        header.header_length = if kind == DATA_HEADER {
            (SDPCM_HEADER_LEN + 2) as u8
        } else {
            SDPCM_HEADER_LEN as u8
        };
        header.wireless_flow_control = 0;
        header.bus_data_credit = 0;
        header.reserved = [0; 2];

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
                let ioctl_offset = header.header_length as usize;

                if size < ioctl_offset + IOCTL_HEADER_LEN {
                    defmt::warn!("CYW43: control packet too small");
                    return Ok(SdpcmPacket::None);
                }

                let flags = u32::from_le_bytes([
                    self.spid_buf[ioctl_offset + 8],
                    self.spid_buf[ioctl_offset + 9],
                    self.spid_buf[ioctl_offset + 10],
                    self.spid_buf[ioctl_offset + 11],
                ]);

                let status = i32::from_le_bytes([
                    self.spid_buf[ioctl_offset + 12],
                    self.spid_buf[ioctl_offset + 13],
                    self.spid_buf[ioctl_offset + 14],
                    self.spid_buf[ioctl_offset + 15],
                ]);

                let id = (flags & CDCF_IOC_ID_MASK) >> CDCF_IOC_ID_SHIFT;

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
                    status,
                });
            }

            DATA_HEADER => {
                if size <= SDPCM_HEADER_LEN + BDC_HEADER_LEN {
                    defmt::warn!("CYW43: data packet too small");
                    return Ok(SdpcmPacket::None);
                }

                let bdc_offset = header.header_length as usize;

                if size < bdc_offset + BDC_HEADER_LEN {
                    defmt::warn!(
                        "CYW43: bad data header size={} bdc_offset={}",
                        size,
                        bdc_offset
                    );
                    return Ok(SdpcmPacket::None);
                }

                let flags2 = self.spid_buf[bdc_offset + 2];
                let data_offset_words = self.spid_buf[bdc_offset + 3] as usize;

                let payload_offset = bdc_offset + BDC_HEADER_LEN + (data_offset_words << 2);

                if size < payload_offset {
                    defmt::warn!(
                        "CYW43: bad data payload size={} payload_offset={}",
                        size,
                        payload_offset
                    );
                    return Ok(SdpcmPacket::None);
                }

                let payload_len = size - payload_offset;

                defmt::info!(
                    "CYW43: DATA size={} hlen={} bdc_off={} data_off_words={} payload_len={}",
                    size,
                    header.header_length,
                    bdc_offset,
                    data_offset_words,
                    payload_len,
                );

                Ok(SdpcmPacket::Data {
                    offset: payload_offset,
                    len: payload_len,
                    interface: Interface::from_u8(flags2),
                })
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
                        defmt::debug!(
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
                    defmt::debug!("CYW43: [SCAN] complete");
                } else {
                    defmt::warn!("CYW43: [SCAN] escan status={}", status);
                }
            }
            event::AUTH => {
                defmt::info!("CYW43: [AUTH] status={}", status);
            }
            event::LINK => {
                defmt::info!("CYW43: [LINK] status={}", status);
            }
            event::PSK_SUP => {
                defmt::info!("CYW43: [PSK_SUP] status={}", status);
            }
            event::SET_SSID => {
                defmt::info!("CYW43: [SET_SSID] status={}", status);

                if status == 0 {
                    self.state = WifiState::Connected;
                } else {
                    self.state = WifiState::ConnectFailed;
                }
            }
            event::DISASSOC => {
                defmt::info!("CYW43: [DISASSOC] status={} reason={}", status, reason);

                self.state = WifiState::Down;
            }
            _ => {
                defmt::debug!("CYW43: [EVENT] unhandled type={}", event_type);
            }
        }

        Ok(())
    }

    fn handle_data_packet(
        &mut self,
        offset: usize,
        len: usize,
        interface: Interface,
    ) -> Option<(usize, usize)> {
        if len < 14 {
            defmt::warn!("CYW43: short ethernet frame len={}", len);
            return None;
        }

        let frame_offset = offset;
        let frame_len = len;

        let ethertype = u16::from_be_bytes([
            self.spid_buf[frame_offset + 12],
            self.spid_buf[frame_offset + 13],
        ]);

        defmt::info!(
            "CYW43: RX DATA iface={} frame_len={} ethertype=0x{:04x}",
            interface as u8,
            frame_len,
            ethertype
        );

        Some((frame_offset, frame_len))
    }

    fn handle_control_packet(&mut self, offset: usize, len: usize, status: i32) {
        self.pending_ioctl_resp = Some(PendingIoctlResp {
            buf: PendingBuf { offset, len },
            status,
        });
        defmt::info!("CYW43: ioctl response stored status={} len={}", status, len);
    }

    pub(super) fn poll(&mut self) -> Result<WlanPollResult, DevError> {
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
                    if let Some((frame_offset, frame_len)) =
                        self.handle_data_packet(offset, len, interface)
                    {
                        self.pending_rx = Some(PendingBuf {
                            offset: frame_offset,
                            len: frame_len,
                        });
                        return Ok(WlanPollResult::Rx);
                    }
                }

                SdpcmPacket::Control {
                    offset,
                    len,
                    status,
                } => {
                    defmt::warn!("CYW43: poll got unexpected control packet");
                    self.handle_control_packet(offset, len, status);
                }

                SdpcmPacket::None => {
                    break;
                }

                SdpcmPacket::Unexpected(kind) => {
                    defmt::warn!("CYW43: [RX] unexpected packet {}", kind);
                }
            }
        }
        Ok(WlanPollResult::None)
    }
}
