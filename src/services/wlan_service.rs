#![allow(dead_code)]
use core::net::Ipv4Addr;

use heapless::{Deque, Vec};
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    socket::{
        dhcpv4::{Event as Dhcpv4Event, Socket as Dhcpv4Socket},
        dns::{self, StartQueryError},
        icmp::{self, Endpoint as IcmpEndpoint, Socket as IcmpSocket},
    },
    time::Instant,
    wire::{EthernetAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr},
};

use crate::{
    drivers::wlan::cyw43::cyw43_country::*,
    net::{
        NetDevice, NetStack, PacketHandle, ScanResult, WifiAuth, WifiConnectFailure, WifiState,
        WlanPollResult, wlan,
    },
    sys::{
        device_driver::DevError,
        syscall::{self, sleep_ms},
    },
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
    ConnectFailed(WifiConnectFailure),
    DhcpConfigured(Ipv4Config),
    DhcpDeconfigured,
    Dns(DnsEvent),
    Ping(PingEvent),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WifiServiceState {
    Off,
    Starting,
    Ready,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WlanServiceError {
    WifiOff,
    NotReady,
    NetworkDown,
    Busy,
    Timeout,
    Driver(DevError),
    InvalidState,
    InvalidPacket,
    InvalidArgument,
    NoAddress,
    NoGateway,
    NoDnsServer,
    Dns(StartQueryError),
    IcmpBindFailed,
    IcmpSendFailed,
    QueueFull,
}

#[derive(Clone, Copy, Debug)]
pub enum DnsEvent {
    Resolved { addr: Ipv4Addr },
    Failed,
    Timeout,
    NetworkDown,
}

#[derive(Clone, Copy)]
enum DnsState {
    Idle,
    Waiting {
        query: dns::QueryHandle,
        started_tick: u64,
    },
    Done(DnsEvent),
}

static NETDEV: NetDevice = NetDevice::new();

static mut SOCKET_STORAGE: [SocketStorage; 4] = [SocketStorage::EMPTY; 4];
static mut DNS_QUERIES: [Option<dns::DnsQuery>; 1] = [None];

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
    Reply {
        addr: Ipv4Addr,
        seq: u16,
        len: usize,
        rtt_ms: u64,
    },
    Timeout {
        addr: Ipv4Addr,
        seq: u16,
    },
    SendFailed {
        addr: Ipv4Addr,
    },
    NetworkDown {
        addr: Ipv4Addr,
        seq: u16,
    },
}

#[derive(Clone, Copy, Debug)]
enum PingState {
    Idle,
    Waiting {
        target: Ipv4Addr,
        seq: u16,
        sent_tick: u64,
    },
    Done(PingEvent),
}

const DNS_TIMEOUT_MS: u64 = 5000;
const PING_TIMEOUT_MS: u64 = 3000;

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
    dns_handle: Option<SocketHandle>,
    dns_state: DnsState,
    wifi_last_state: WifiState,
    wifi_is_on: bool,
    rx_buf: [u8; 1536],
    ping_seq: u16,
    ping_state: PingState,
    dhcp_configured: bool,
    pending_tx: Option<PacketHandle>,
    pending_events: Deque<WlanServiceEvent, 8>,
    pending_connect_failure: Option<WifiConnectFailure>,
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

        // DNS socket. DHCP will update the server list after configuration.
        let dns_socket = dns::Socket::new(&[], unsafe { &mut DNS_QUERIES[..] });
        let dns_handle = sockets.add(dns_socket);

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
            dns_handle: Some(dns_handle),
            dns_state: DnsState::Idle,
            wifi_last_state: WifiState::Down,
            wifi_is_on: false,
            rx_buf: [0; 1536],
            ping_seq: 0,
            ping_state: PingState::Idle,
            dhcp_configured: false,
            pending_tx: None,
            pending_events: Deque::new(),
            pending_connect_failure: None,
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

    pub fn wifi_on(&mut self) -> Result<(), WlanServiceError> {
        if self.wifi_is_on {
            defmt::info!("WLANSRV: wifi is already on");
            return Ok(());
        }

        defmt::info!("WLANSRV: wifi on");

        // clear
        self.iface = None;
        self.mac = None;
        self.ip = None;
        self.gateway = None;
        self.dns = None;
        self.dhcp_configured = false;

        wlan().wifi_on(CYW43_COUNTRY_CANADA, None).map_err(|e| {
            defmt::warn!("WLANSRV: wifi on failed");
            WlanServiceError::Driver(e)
        })?;

        let mac = match wlan().get_mac_addr() {
            Ok(mac) => mac,
            Err(e) => {
                defmt::warn!("WLANSRV: get MAC address failed");

                // let _ = wlan().wifi_off();

                return Err(WlanServiceError::Driver(e));
            }
        };

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

        self.wifi_is_on = true;

        for _ in 0..100 {
            match wlan().poll() {
                Ok(_) => {}
                Err(e) => {
                    defmt::warn!("WLANSRV: poll during startup failed: {:?}", e as usize);
                }
            }
            sleep_ms(10);
        }

        Ok(())
    }

    fn wifi_status(&mut self) -> Result<WifiState, WlanServiceError> {
        if !self.wifi_is_on {
            return Ok(WifiState::Down);
        }
        wlan().wifi_status().map_err(WlanServiceError::Driver)
    }

    fn clear_network_state(&mut self) {
        self.cancel_pending_dns(DnsEvent::NetworkDown);
        self.cancel_pending_ping();

        self.dhcp_configured = false;
        self.ip = None;
        self.gateway = None;
        self.dns = None;

        if let Some(handle) = self.dns_handle {
            self.sockets
                .get_mut::<dns::Socket>(handle)
                .update_servers(&[]);
        }

        if let Some(iface) = self.iface.as_mut() {
            iface.update_ip_addrs(|addrs| {
                addrs.clear();
            });

            let _ = iface.routes_mut().remove_default_ipv4_route();
        }
    }

    pub fn wifi_scan(&mut self, timeout_ms: u32) -> Result<Vec<ScanResult, 32>, WlanServiceError> {
        if !self.wifi_is_on {
            defmt::warn!("WLANSRV: wifi is off");
            return Err(WlanServiceError::WifiOff);
        }

        defmt::info!("WLANSRV: wifi scan requested");

        wlan().wifi_scan().map_err(|e| {
            defmt::warn!("WLANSRV: wifi scan start failed");
            WlanServiceError::Driver(e)
        })?;

        let started_tick = syscall::get_tick();

        loop {
            wlan().poll().map_err(|e| {
                defmt::warn!("WLANSRV: poll during scan failed");
                WlanServiceError::Driver(e)
            })?;

            match wlan().wifi_scan_done() {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    defmt::warn!("WLANSRV: failed to query scan status");
                    return Err(WlanServiceError::Driver(e));
                }
            }

            let elapsed = syscall::get_tick().wrapping_sub(started_tick);

            if elapsed >= timeout_ms as u64 {
                defmt::warn!("WLANSRV: wifi scan timeout");
                return Err(WlanServiceError::Timeout);
            }

            sleep_ms(10);
        }

        let mut results = Vec::new();

        wlan().wifi_scan_results(&mut results).map_err(|e| {
            defmt::warn!("WLANSRV: failed to get scan results");
            WlanServiceError::Driver(e)
        })?;

        Ok(results)
    }

    pub fn wifi_connect(
        &mut self,
        ssid: FixedStr<32>,
        password: Option<FixedStr<64>>,
        auth: WifiAuth,
    ) -> Result<(), WlanServiceError> {
        match self.wifi_status()? {
            WifiState::Connected => {
                defmt::warn!("WLANSRV: already connected");
                return Ok(());
            }
            WifiState::Connecting => {
                defmt::warn!("WLANSRV: connection already in progress");
                return Err(WlanServiceError::Busy);
            }
            WifiState::Disconnecting => {
                defmt::warn!("WLANSRV: disconnection already in progress");
                return Err(WlanServiceError::Busy);
            }
            WifiState::Down => {}
        }

        defmt::info!("WLANSRV: wifi connect requested");

        let password = password.as_ref().map_or("", |pw| pw.as_str());

        wlan()
            .wifi_connect(ssid.as_str(), password, auth)
            .map_err(WlanServiceError::Driver)?;

        Ok(())
    }

    pub fn wifi_disconnect(&mut self) -> Result<(), WlanServiceError> {
        defmt::info!("WLANSRV: wifi disconnect requested");

        match self.wifi_status()? {
            WifiState::Down => {
                defmt::info!("WLANSRV: wifi is already disconnected");
            }
            _ => {
                wlan().wifi_disconnect().map_err(|e| {
                    defmt::warn!("WLANSRV: wifi disconnect failed");
                    WlanServiceError::Driver(e)
                })?;
            }
        }

        self.clear_network_state();

        Ok(())
    }

    pub fn poll(&mut self) {
        for _ in 0..16 {
            match self.poll_wifi() {
                Ok(true) => {
                    self.poll_smoltcp();
                }

                Ok(false) => {
                    break;
                }

                Err(_e) => {
                    defmt::warn!("WLANSRV: poll_wifi failed.");
                    break;
                }
            }
        }

        self.poll_smoltcp();
        self.poll_state_events();
    }

    fn poll_state_events(&mut self) {
        match self.poll_wifi_state_change() {
            Ok(Some((old, new))) => {
                defmt::info!(
                    "WLANSRV: wifi state changed: {:?} -> {:?}",
                    old as usize,
                    new as usize
                );

                match new {
                    WifiState::Connected => {
                        self.push_event(WlanServiceEvent::LinkConnected);
                    }

                    WifiState::Down => {
                        self.clear_network_state();

                        if let Some(failure) = self.pending_connect_failure.take() {
                            self.push_event(WlanServiceEvent::ConnectFailed(failure));
                        } else if old == WifiState::Connected {
                            self.push_event(WlanServiceEvent::LinkDisconnected);
                        }
                    }

                    WifiState::Connecting | WifiState::Disconnecting => {}
                }
            }

            Ok(None) => {}

            Err(WlanServiceError::Driver(e)) => {
                defmt::warn!("WLANSRV: failed to poll wifi state: {}", e as usize);
            }

            Err(_) => {
                defmt::warn!("WLANSRV: failed to poll wifi state");
            }
        }

        if let Some(event) = self.take_dns_event() {
            self.push_event(WlanServiceEvent::Dns(event));
        }

        if let Some(event) = self.take_ping_event() {
            self.push_event(WlanServiceEvent::Ping(event));
        }
    }

    fn poll_wifi(&mut self) -> Result<bool, WlanServiceError> {
        match wlan().poll().map_err(WlanServiceError::Driver)? {
            WlanPollResult::Rx => {
                let len = wlan().get_rx_buf(&mut self.rx_buf).map_err(|e| {
                    defmt::warn!("WLANSRV: get_rx_buf failed: {:?}", e as usize);
                    WlanServiceError::Driver(e)
                })?;

                if len > self.rx_buf.len() {
                    defmt::warn!(
                        "WLANSRV: invalid RX length: {}, buffer size: {}",
                        len,
                        self.rx_buf.len()
                    );

                    return Err(WlanServiceError::InvalidPacket);
                }

                if !NETDEV.inject_rx(&self.rx_buf[..len]) {
                    defmt::warn!("WLANSRV: NETDEV inject_rx failed len={}", len);
                }

                Ok(true)
            }

            WlanPollResult::ConnectFailed(failure) => {
                self.clear_network_state();

                match wlan().wifi_abort_connect() {
                    Ok(()) => {
                        self.pending_connect_failure = Some(failure);
                    }

                    Err(e) => {
                        defmt::warn!("WLANSRV: abort failed connection failed: {}", e as usize);

                        self.push_event(WlanServiceEvent::ConnectFailed(failure));
                    }
                }

                Ok(true)
            }

            WlanPollResult::None => Ok(false),
        }
    }

    fn poll_wifi_state_change(
        &mut self,
    ) -> Result<Option<(WifiState, WifiState)>, WlanServiceError> {
        let old = self.wifi_last_state;
        let new = self.wifi_status()?;

        if old == new {
            return Ok(None);
        }

        self.wifi_last_state = new;
        Ok(Some((old, new)))
    }

    fn poll_smoltcp(&mut self) {
        match self.wifi_status() {
            Ok(WifiState::Connected) => {}
            Ok(_) => return,
            Err(WlanServiceError::Driver(e)) => {
                defmt::warn!("WLANSRV: wifi_status failed: {}", e as usize);
                return;
            }
            Err(_) => {
                defmt::warn!("WLANSRV: wifi_status failed");
                return;
            }
        }

        self.iface_poll_once();

        if let Err(_e) = self.drain_tx() {
            defmt::warn!("WLANSRV: drain_tx failed");
        }

        self.icmp_poll();
        self.dhcp_poll();
        self.dns_poll();

        self.iface_poll_once();

        if let Err(_e) = self.drain_tx() {
            defmt::warn!("WLANSRV: drain_tx failed");
        }
    }

    fn iface_poll_once(&mut self) {
        if let Some(iface) = self.iface.as_mut() {
            let now = Instant::from_millis(syscall::get_tick() as i64);
            let _ = iface.poll(now, &mut self.smol_dev, &mut self.sockets);
        }
    }

    pub fn resolve(&mut self, hostname: &str) -> Result<(), WlanServiceError> {
        if !matches!(self.dns_state, DnsState::Idle) {
            return Err(WlanServiceError::Busy);
        }

        if self.dns.is_none() {
            return Err(WlanServiceError::NoDnsServer);
        }

        if hostname.is_empty() {
            return Err(WlanServiceError::InvalidArgument);
        }

        let handle = self.dns_handle.ok_or(WlanServiceError::NotReady)?;

        let iface = self.iface.as_mut().ok_or(WlanServiceError::NotReady)?;

        let socket = self.sockets.get_mut::<dns::Socket>(handle);

        let query = socket
            .start_query(iface.context(), hostname, smoltcp::wire::DnsQueryType::A)
            .map_err(|e| {
                defmt::warn!("WLANSRV: DNS query start failed: {}", e as i32);
                WlanServiceError::Dns(e)
            })?;

        self.dns_state = DnsState::Waiting {
            query,
            started_tick: syscall::get_tick(),
        };

        Ok(())
    }

    pub fn ping(&mut self, target: Ipv4Addr) -> Result<(), WlanServiceError> {
        if !matches!(self.ping_state, PingState::Idle) {
            return Err(WlanServiceError::Busy);
        }

        let seq = self.ping_seq.wrapping_add(1);

        if !self.dhcp_configured || self.ip.is_none() {
            self.ping_seq = seq;

            self.ping_state = PingState::Done(PingEvent::NetworkDown { addr: target, seq });

            return Ok(());
        }

        let socket = self.sockets.get_mut::<IcmpSocket>(self.icmp_handle);

        if !socket.is_open() {
            socket.bind(IcmpEndpoint::Ident(0x1234)).map_err(|_| {
                defmt::warn!("WLANSRV: ICMP bind failed");
                WlanServiceError::IcmpBindFailed
            })?;
        }

        let echo = Icmpv4Repr::EchoRequest {
            ident: 0x1234,
            seq_no: seq,
            data: b"miniRTOS",
        };

        let payload_len = echo.buffer_len();

        let buf = socket
            .send(payload_len, IpAddress::Ipv4(target))
            .map_err(|_| {
                defmt::warn!("WLANSRV: ICMP send failed");
                WlanServiceError::IcmpSendFailed
            })?;

        let mut packet = Icmpv4Packet::new_unchecked(buf);

        echo.emit(&mut packet, &smoltcp::phy::ChecksumCapabilities::default());

        self.ping_seq = seq;

        self.ping_state = PingState::Waiting {
            target,
            seq,
            sent_tick: syscall::get_tick(),
        };

        Ok(())
    }

    pub fn ping_gateway(&mut self) -> Result<(), WlanServiceError> {
        let Some(gw) = self.gateway else {
            defmt::warn!("WLANSRV: no gateway");
            return Err(WlanServiceError::NoGateway);
        };

        self.ping(gw)
    }

    fn cancel_pending_ping(&mut self) {
        let waiting = match &self.ping_state {
            PingState::Waiting { target, seq, .. } => Some((*target, *seq)),

            _ => None,
        };

        if let Some((target, seq)) = waiting {
            self.ping_state = PingState::Done(PingEvent::NetworkDown { addr: target, seq });
        }
    }

    fn icmp_poll(&mut self) {
        let socket = self.sockets.get_mut::<IcmpSocket>(self.icmp_handle);

        while socket.can_recv() {
            let Ok((data, endpoint)) = socket.recv() else {
                break;
            };

            let IpAddress::Ipv4(source) = endpoint;

            let Ok(packet) = Icmpv4Packet::new_checked(data) else {
                defmt::warn!("WLANSRV: invalid ICMP packet");
                continue;
            };

            let Ok(repr) =
                Icmpv4Repr::parse(&packet, &smoltcp::phy::ChecksumCapabilities::default())
            else {
                defmt::warn!("WLANSRV: ICMP parse failed");
                continue;
            };

            let Icmpv4Repr::EchoReply {
                ident,
                seq_no,
                data,
            } = repr
            else {
                continue;
            };

            let PingState::Waiting {
                target,
                seq,
                sent_tick,
            } = self.ping_state
            else {
                continue;
            };

            if ident != 0x1234 || seq_no != seq || source != target {
                continue;
            }

            let rtt_ms = syscall::get_tick().wrapping_sub(sent_tick);

            self.ping_state = PingState::Done(PingEvent::Reply {
                addr: source,
                seq,
                len: data.len(),
                rtt_ms,
            });
            break;
        }

        let timeout = match &self.ping_state {
            PingState::Waiting {
                target,
                seq,
                sent_tick,
            } => {
                let elapsed_ms = syscall::get_tick().wrapping_sub(*sent_tick);

                if elapsed_ms >= PING_TIMEOUT_MS {
                    Some((*target, *seq))
                } else {
                    None
                }
            }

            _ => None,
        };

        if let Some((target, seq)) = timeout {
            self.ping_state = PingState::Done(PingEvent::Timeout { addr: target, seq });
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

    fn dns_poll(&mut self) {
        let (query, started_tick) = match &self.dns_state {
            DnsState::Waiting {
                query,
                started_tick,
            } => (*query, *started_tick),

            _ => return,
        };

        let Some(handle) = self.dns_handle else {
            defmt::error!("WLANSRV: DNS query is waiting, but DNS socket is missing");

            self.dns_state = DnsState::Done(DnsEvent::Failed);
            return;
        };

        let result = {
            let socket = self.sockets.get_mut::<dns::Socket>(handle);
            socket.get_query_result(query)
        };

        match result {
            Ok(addrs) => {
                let ipv4_addr = addrs.iter().find_map(|addr| match addr {
                    IpAddress::Ipv4(addr) => Some(*addr),
                });

                self.dns_state = match ipv4_addr {
                    Some(addr) => {
                        let b = addr.octets();
                        defmt::info!("WLANSRV: DNS resolved: {}.{}.{}.{}", b[0], b[1], b[2], b[3],);

                        DnsState::Done(DnsEvent::Resolved { addr })
                    }

                    None => {
                        defmt::warn!("WLANSRV: DNS query completed without IPv4 address");

                        DnsState::Done(DnsEvent::Failed)
                    }
                };
            }

            Err(dns::GetQueryResultError::Pending) => {
                let elapsed_ms = syscall::get_tick().wrapping_sub(started_tick);

                if elapsed_ms < DNS_TIMEOUT_MS {
                    return;
                }

                defmt::warn!("WLANSRV: DNS query timeout");

                {
                    let socket = self.sockets.get_mut::<dns::Socket>(handle);

                    socket.cancel_query(query);
                }

                self.dns_state = DnsState::Done(DnsEvent::Timeout);
            }

            Err(dns::GetQueryResultError::Failed) => {
                defmt::warn!("WLANSRV: DNS query failed");

                self.dns_state = DnsState::Done(DnsEvent::Failed);
            }
        }
    }

    fn take_dns_event(&mut self) -> Option<DnsEvent> {
        match self.dns_state {
            DnsState::Done(event) => {
                self.dns_state = DnsState::Idle;
                Some(event)
            }
            _ => None,
        }
    }

    fn cancel_dns_query(&mut self, event: DnsEvent) {
        let query = match &self.dns_state {
            DnsState::Waiting { query, .. } => *query,
            _ => return,
        };

        if let Some(handle) = self.dns_handle {
            let socket = self.sockets.get_mut::<dns::Socket>(handle);

            socket.cancel_query(query);
        }

        self.dns_state = DnsState::Done(event);
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
                /*
                 * DHCP may renew with a different DNS server or address.
                 * Cancel any query that was started using the previous
                 * configuration.
                 */
                self.cancel_pending_dns(DnsEvent::Failed);

                self.dhcp_configured = true;
                self.ip = Some(config.address);
                self.gateway = config.gateway;
                self.dns = config.dns;

                if let Some(handle) = self.dns_handle {
                    let socket = self.sockets.get_mut::<dns::Socket>(handle);

                    match config.dns {
                        Some(server) => {
                            socket.update_servers(&[IpAddress::Ipv4(server)]);
                        }

                        None => {
                            socket.update_servers(&[]);
                            defmt::warn!("WLANSRV: DHCP did not provide DNS server");
                        }
                    }
                }

                if let Some(iface) = self.iface.as_mut() {
                    iface.update_ip_addrs(|addrs| {
                        addrs.clear();

                        let cidr = smoltcp::wire::Ipv4Cidr::new(config.address, config.prefix_len);

                        if addrs.push(IpCidr::Ipv4(cidr)).is_err() {
                            defmt::warn!("WLANSRV: failed to add interface address");
                        }
                    });

                    let _ = iface.routes_mut().remove_default_ipv4_route();

                    match config.gateway {
                        Some(gateway) => {
                            if iface.routes_mut().add_default_ipv4_route(gateway).is_err() {
                                defmt::warn!("WLANSRV: failed to add default route");
                            }
                        }

                        None => {
                            defmt::warn!("WLANSRV: DHCP did not provide gateway");
                        }
                    }
                }

                defmt::info!("WLANSRV: DHCP configured");

                self.push_event(WlanServiceEvent::DhcpConfigured(config));
            }

            Some(WlanServiceEvent::DhcpDeconfigured) => {
                defmt::warn!("WLANSRV: DHCP deconfigured");

                self.cancel_pending_dns(DnsEvent::Failed);
                self.cancel_pending_ping();

                self.dhcp_configured = false;
                self.ip = None;
                self.gateway = None;
                self.dns = None;

                if let Some(handle) = self.dns_handle {
                    self.sockets
                        .get_mut::<dns::Socket>(handle)
                        .update_servers(&[]);
                }

                if let Some(iface) = self.iface.as_mut() {
                    iface.update_ip_addrs(|addrs| {
                        addrs.clear();
                    });

                    let _ = iface.routes_mut().remove_default_ipv4_route();
                }

                self.push_event(WlanServiceEvent::DhcpDeconfigured);
            }

            _ => {}
        }
    }

    fn cancel_pending_dns(&mut self, event: DnsEvent) {
        let query = match &self.dns_state {
            DnsState::Waiting { query, .. } => Some(*query),
            _ => None,
        };

        let Some(query) = query else {
            return;
        };

        if let Some(handle) = self.dns_handle {
            let socket = self.sockets.get_mut::<dns::Socket>(handle);

            socket.cancel_query(query);
        }

        self.dns_state = DnsState::Done(event);
    }

    fn drain_tx(&mut self) -> Result<(), WlanServiceError> {
        loop {
            let handle = match self.pending_tx.take() {
                Some(handle) => handle,
                None => match NETDEV.take_tx() {
                    Some(handle) => handle,
                    None => return Ok(()),
                },
            };

            let result = NETDEV.with_packet(handle, |pkt| wlan().sent_tx_buf(pkt));

            match result {
                Some(Ok(())) => {
                    NETDEV.free_packet(handle);
                }

                Some(Err(e)) if matches!(e, DevError::Busy | DevError::Timeout) => {
                    self.pending_tx = Some(handle);
                    return Ok(());
                }

                Some(Err(e)) => {
                    NETDEV.free_packet(handle);
                    return Err(WlanServiceError::Driver(e));
                }

                None => {
                    NETDEV.free_packet(handle);
                    return Err(WlanServiceError::InvalidPacket);
                }
            }
        }
    }

    fn clear_pending_tx(&mut self) {
        if let Some(handle) = self.pending_tx.take() {
            NETDEV.free_packet(handle);
        }
    }
}
