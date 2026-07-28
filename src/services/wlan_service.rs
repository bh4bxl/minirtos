#![allow(dead_code)]
use core::net::Ipv4Addr;

use heapless::{Deque, Vec};
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    socket::{
        dhcpv4::{Event as Dhcpv4Event, Socket as Dhcpv4Socket},
        icmp::{self, Endpoint as IcmpEndpoint, Socket as IcmpSocket},
    },
    time::Instant,
    wire::{EthernetAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr},
};

use crate::{
    drivers::wlan::cyw43::cyw43_country::*,
    net::{NetDevice, NetStack, ScanResult, WifiAuth, WifiState, WlanPollResult, wlan},
    sys::syscall::{self, sleep_ms},
};

#[derive(Clone, Copy, Debug)]
pub struct Ipv4Config {
    pub address: Ipv4Addr,
    pub prefix_len: u8,
    pub gateway: Option<Ipv4Addr>,
    pub dns: Option<Ipv4Addr>,
}

#[derive(Clone, Copy, Debug)]
pub enum WlanServiceEvent {
    LinkConnected,
    LinkDisconnected,
    ConnectFailed,
    DhcpConfigured(Ipv4Config),
    DhcpDeconfigured,
    Ping(PingEvent),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WifiServiceState {
    Off,
    Starting,
    Ready,
}

static NETDEV: NetDevice = NetDevice::new();

static mut SOCKET_STORAGE: [SocketStorage; 4] = [SocketStorage::EMPTY; 4];

#[derive(Clone, Copy)]
pub struct FixedStr<const N: usize> {
    pub buf: [u8; N],
    pub len: usize,
}

impl<const N: usize> FixedStr<N> {
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() > N {
            return None;
        }

        let mut out = Self {
            buf: [0; N],
            len: s.len(),
        };

        out.buf[..s.len()].copy_from_slice(s.as_bytes());
        Some(out)
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len]) }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PingEvent {
    Reply { seq: u16, len: usize, rtt_ms: u32 },
    Timeout { seq: u16 },
}

#[derive(Clone, Copy, Debug)]
enum PingState {
    Idle,
    Waiting { seq: u16, sent_tick: u64 },
    Done(PingEvent),
}

pub struct WlanService {
    iface: Option<Interface>,
    smol_dev: NetStack,
    sockets: SocketSet<'static>,
    dhcp_handle: SocketHandle,
    icmp_handle: SocketHandle,
    mac: Option<EthernetAddress>,
    ip: Option<Ipv4Addr>,
    gateway: Option<Ipv4Addr>,
    dns: Option<Ipv4Addr>,
    wifi_last_state: WifiState,
    wifi_is_on: bool,
    rx_buf: [u8; 1536],
    ping_seq: u16,
    ping_state: PingState,
    dhcp_configured: bool,
    pending_events: Deque<WlanServiceEvent, 8>,
}

impl WlanService {
    pub fn new() -> Self {
        let smol_dev = NetStack::new(&NETDEV);

        // socket storage
        let mut sockets = SocketSet::new(unsafe { &mut SOCKET_STORAGE[..] });

        // DHCP socket
        let dhcp_handle = sockets.add(Dhcpv4Socket::new());

        // ICMP socket
        static mut ICMP_RX_META: [icmp::PacketMetadata; 8] = [icmp::PacketMetadata::EMPTY; 8];
        static mut ICMP_TX_META: [icmp::PacketMetadata; 8] = [icmp::PacketMetadata::EMPTY; 8];

        static mut ICMP_RX_BUF: [u8; 256] = [0; 256];
        static mut ICMP_TX_BUF: [u8; 256] = [0; 256];
        let icmp_socket = icmp::Socket::new(
            icmp::PacketBuffer::new(unsafe { &mut ICMP_RX_META[..] }, unsafe {
                &mut ICMP_RX_BUF[..]
            }),
            icmp::PacketBuffer::new(unsafe { &mut ICMP_TX_META[..] }, unsafe {
                &mut ICMP_TX_BUF[..]
            }),
        );
        let icmp_handle = sockets.add(icmp_socket);
        Self {
            iface: None,
            smol_dev,
            sockets,
            dhcp_handle,
            icmp_handle,
            mac: None,
            ip: None,
            gateway: None,
            dns: None,
            wifi_last_state: WifiState::Down,
            wifi_is_on: false,
            rx_buf: [0; 1536],
            ping_seq: 0,
            ping_state: PingState::Idle,
            dhcp_configured: false,
            pending_events: Deque::new(),
        }
    }

    fn push_event(&mut self, event: WlanServiceEvent) {
        if self.pending_events.push_back(event).is_err() {
            defmt::warn!("WLANSRV: event queue full");
        }
    }

    pub fn take_event(&mut self) -> Option<WlanServiceEvent> {
        self.pending_events.pop_front()
    }

    pub fn wifi_on(&mut self) {
        if self.wifi_is_on {
            defmt::info!("WLANSRV: wifi is already on");
            return;
        }

        defmt::info!("WLANSRV: wifi on");

        if !self.wifi_is_on {
            if wlan().wifi_on(CYW43_COUNTRY_CANADA, None).is_err() {
                defmt::warn!("WLANSRV: wifi on failed");
                return;
            }
            if let Ok(mac) = wlan().get_mac_addr() {
                self.mac = Some(EthernetAddress(mac));

                defmt::info!(
                    "WLANSRV: wlan mac: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    mac[0],
                    mac[1],
                    mac[2],
                    mac[3],
                    mac[4],
                    mac[5],
                );

                let mut config = Config::new(self.mac.unwrap().into());
                config.random_seed = 0x1234_5678;

                self.iface = Some(Interface::new(
                    config,
                    &mut self.smol_dev,
                    Instant::from_millis(syscall::get_tick() as i64),
                ));
            }
            self.ip = None;
            self.gateway = None;
            self.dns = None;

            self.wifi_is_on = true;

            for _ in 0..100 {
                let _ = wlan().poll();
                sleep_ms(10);
            }
        }
    }

    pub fn wifi_status(&mut self) -> WifiState {
        if !self.wifi_is_on {
            WifiState::Down
        } else {
            wlan().wifi_status().unwrap_or(WifiState::Down)
        }
    }

    pub fn wifi_scan(&mut self, timeout: u32) -> Option<Vec<ScanResult, 32>> {
        if !self.wifi_is_on {
            defmt::warn!("WLANSRV: wifi is off");
            return None;
        }

        defmt::info!("WLANSRV: wifi scan requested");

        if wlan().wifi_scan().is_ok() {
            let mut remain = timeout;

            loop {
                let _ = wlan().poll();

                if wlan().wifi_scan_done().unwrap() {
                    break;
                }

                sleep_ms(10);

                if remain == 0 {
                    return None;
                }

                remain = remain.saturating_sub(10);
            }
            let mut res = heapless::Vec::new();
            wlan().wifi_scan_results(&mut res).unwrap();
            Some(res)
        } else {
            defmt::warn!("WLANSRV: wifi scan failed");
            None
        }
    }

    pub fn wifi_connect(
        &mut self,
        ssid: FixedStr<32>,
        password: Option<FixedStr<64>>,
        auth: WifiAuth,
    ) -> bool {
        let wifi_state = self.wifi_status();

        match wifi_state {
            WifiState::Connected => {
                defmt::warn!("WLANSRV: already connected");
                return true;
            }
            WifiState::Connecting => {
                defmt::warn!("WLANSRV: connection already in progress");
                return true;
            }
            WifiState::Down | WifiState::ConnectFailed => {}
        }

        defmt::info!("WLANSRV: wifi connect requested");

        let password_str = match &password {
            Some(pw) => pw.as_str(),
            None => "",
        };

        if wlan()
            .wifi_connect(ssid.as_str(), password_str, auth)
            .is_err()
        {
            defmt::warn!("WLANSRV: wifi connect failed");
            return false;
        }

        true
    }

    pub fn wifi_disconnect(&mut self) {
        defmt::info!("WLANSRV: wifi disconnect requested");
        let wifi_state = self.wifi_status();

        if wifi_state != WifiState::Down {
            if wlan().wifi_disconnect().is_err() {
                defmt::warn!("WLANSRV: wifi disconnect failed");
            }
        }

        self.dhcp_configured = false;

        self.ip = None;
        self.gateway = None;
        self.dns = None;
    }

    pub fn poll(&mut self) {
        for _ in 0..16 {
            if !self.poll_wifi() {
                break;
            }

            self.poll_smoltcp();
        }

        self.poll_smoltcp();
        self.poll_state_events();
    }

    fn poll_state_events(&mut self) {
        if let Some((_old, new)) = self.poll_wifi_state_change() {
            match new {
                WifiState::Connected => self.push_event(WlanServiceEvent::LinkConnected),
                WifiState::ConnectFailed => self.push_event(WlanServiceEvent::ConnectFailed),
                WifiState::Down => self.push_event(WlanServiceEvent::LinkDisconnected),
                _ => (),
            }
        }

        if let Some(event) = self.take_ping_event() {
            self.push_event(WlanServiceEvent::Ping(event));
        }
    }

    fn poll_wifi(&mut self) -> bool {
        match wlan().poll() {
            Ok(WlanPollResult::Rx) => {
                match wlan().get_rx_buf(&mut self.rx_buf) {
                    Ok(len) => {
                        if len >= 42 {
                            let ethertype = u16::from_be_bytes([self.rx_buf[12], self.rx_buf[13]]);

                            if ethertype == 0x0800 {
                                let proto = self.rx_buf[23];

                                if proto == 1 && len >= 42 {
                                    let icmp_off = 14 + 20;
                                    let icmp_type = self.rx_buf[icmp_off];
                                    let ident = u16::from_be_bytes([
                                        self.rx_buf[icmp_off + 4],
                                        self.rx_buf[icmp_off + 5],
                                    ]);
                                    let seq = u16::from_be_bytes([
                                        self.rx_buf[icmp_off + 6],
                                        self.rx_buf[icmp_off + 7],
                                    ]);

                                    if icmp_type == 0 && ident == 0x1234 {
                                        if let PingState::Waiting {
                                            seq: pending_seq,
                                            sent_tick,
                                        } = self.ping_state
                                        {
                                            if seq == pending_seq {
                                                let rtt = syscall::get_tick()
                                                    .wrapping_sub(sent_tick)
                                                    as u32;

                                                self.ping_state =
                                                    PingState::Done(PingEvent::Reply {
                                                        seq,
                                                        len: len - 14 - 20,
                                                        rtt_ms: rtt,
                                                    });
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !NETDEV.inject_rx(&self.rx_buf[..len]) {
                            defmt::warn!("WLANSRV: NETDEV inject_rx failed len={}", len);
                        }
                    }

                    Err(e) => {
                        defmt::warn!("WLANSRV: get_rx_buf failed: {:?}", e as usize);
                    }
                }
                true
            }

            Ok(WlanPollResult::None) => false,

            Err(e) => {
                defmt::warn!("WLANSRV: poll failed: {}", e as usize);
                false
            }
        }
    }

    fn poll_wifi_state_change(&mut self) -> Option<(WifiState, WifiState)> {
        let old = self.wifi_last_state;
        let new = self.wifi_status();

        if old == new {
            return None;
        }

        self.wifi_last_state = new;
        Some((old, new))
    }

    fn poll_smoltcp(&mut self) {
        if self.wifi_status() != WifiState::Connected {
            return;
        }

        self.iface_poll_once();
        self.drain_tx();

        self.icmp_poll();
        self.dhcp_poll();

        self.iface_poll_once();
        self.drain_tx();
    }

    fn iface_poll_once(&mut self) {
        if let Some(iface) = self.iface.as_mut() {
            let now = Instant::from_millis(syscall::get_tick() as i64);
            let _ = iface.poll(now, &mut self.smol_dev, &mut self.sockets);
        }
    }

    pub fn ping_gateway(&mut self) -> bool {
        if !matches!(self.ping_state, PingState::Idle) {
            return false;
        }

        let Some(gw) = self.gateway else {
            defmt::warn!("WLANSRV: no gateway");
            return false;
        };

        let socket = self.sockets.get_mut::<IcmpSocket>(self.icmp_handle);

        if !socket.is_open() {
            socket.bind(IcmpEndpoint::Ident(0x1234)).ok();
        }

        self.ping_seq = self.ping_seq.wrapping_add(1);

        let echo = Icmpv4Repr::EchoRequest {
            ident: 0x1234,
            seq_no: self.ping_seq,
            data: b"miniRTOS",
        };

        let payload_len = echo.buffer_len();

        let Ok(mut buf) = socket.send(payload_len, IpAddress::Ipv4(gw)) else {
            defmt::warn!("WLANSRV: ICMP send failed");
            return false;
        };

        let mut packet = Icmpv4Packet::new_unchecked(&mut buf);

        echo.emit(&mut packet, &smoltcp::phy::ChecksumCapabilities::default());

        self.ping_state = PingState::Waiting {
            seq: self.ping_seq,
            sent_tick: syscall::get_tick(),
        };

        true
    }

    fn icmp_poll(&mut self) {
        let socket = self.sockets.get_mut::<IcmpSocket>(self.icmp_handle);

        while socket.can_recv() {
            let _ = socket.recv();
        }

        if let PingState::Waiting { seq, sent_tick } = self.ping_state {
            let elapsed = syscall::get_tick().wrapping_sub(sent_tick);

            if elapsed > 3000 {
                self.ping_state = PingState::Done(PingEvent::Timeout { seq });
            }
        }
    }

    fn take_ping_event(&mut self) -> Option<PingEvent> {
        match self.ping_state {
            PingState::Done(event) => {
                self.ping_state = PingState::Idle;
                Some(event)
            }
            _ => None,
        }
    }

    fn dhcp_poll(&mut self) {
        let event = {
            let dhcp = self.sockets.get_mut::<Dhcpv4Socket>(self.dhcp_handle);

            match dhcp.poll() {
                Some(Dhcpv4Event::Configured(config)) => {
                    Some(WlanServiceEvent::DhcpConfigured(Ipv4Config {
                        address: config.address.address(),
                        prefix_len: config.address.prefix_len(),
                        gateway: config.router,
                        dns: config.dns_servers.first().copied(),
                    }))
                }

                Some(Dhcpv4Event::Deconfigured) => {
                    if self.dhcp_configured {
                        Some(WlanServiceEvent::DhcpDeconfigured)
                    } else {
                        defmt::info!("WLANSRV: ignore initial DHCP Deconfigured");
                        None
                    }
                }

                None => None,
            }
        };

        match event {
            Some(WlanServiceEvent::DhcpConfigured(config)) => {
                self.dhcp_configured = true;

                self.ip = Some(config.address);
                self.gateway = config.gateway;
                self.dns = config.dns;

                if let Some(iface) = self.iface.as_mut() {
                    iface.update_ip_addrs(|addrs| {
                        addrs.clear();

                        let cidr = smoltcp::wire::Ipv4Cidr::new(config.address, config.prefix_len);

                        let _ = addrs.push(IpCidr::Ipv4(cidr));
                    });
                }

                self.push_event(WlanServiceEvent::DhcpConfigured(config));
            }

            Some(WlanServiceEvent::DhcpDeconfigured) => {
                self.dhcp_configured = false;

                self.ip = None;
                self.gateway = None;
                self.dns = None;

                if let Some(iface) = self.iface.as_mut() {
                    iface.update_ip_addrs(|addrs| {
                        addrs.clear();
                    });

                    iface.routes_mut().remove_default_ipv4_route();
                }

                self.push_event(WlanServiceEvent::DhcpDeconfigured);
            }

            _ => {}
        }
    }

    fn drain_tx(&mut self) {
        while let Some(handle) = NETDEV.take_tx() {
            let sent = NETDEV.with_packet(handle, |pkt| wlan().sent_tx_buf(pkt));

            match sent {
                Some(Ok(())) => {}
                Some(Err(e)) => {
                    defmt::warn!("WLANSRV: send_data failed: {:?}", e as usize);
                }
                None => {
                    defmt::warn!("WLANSRV: TX packet missing");
                }
            }

            NETDEV.free_packet(handle);
        }
    }
}
