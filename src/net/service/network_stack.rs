use core::net::Ipv4Addr;

use heapless::Deque;
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    phy::ChecksumCapabilities,
    socket::{
        dhcpv4::{Event as Dhcpv4Event, Socket as Dhcpv4Socket},
        dns::{self, GetQueryResultError, Socket as DnsSocket},
        icmp::{self, Endpoint as IcmpEndpoint, Socket as IcmpSocket},
        tcp::{self, Socket as TcpSocket},
    },
    time::Instant,
    wire::{EthernetAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr, Ipv4Cidr},
};

use crate::sys::syscall;

use super::super::core::NetStack;

use super::*;

const DNS_TIMEOUT_MS: u64 = 5000;
const PING_TIMEOUT_MS: u64 = 3000;
const TCP_CONNECT_TIMEOUT_MS: u64 = 5000;
const TCP_ECHO_TIMEOUT_MS: u64 = 5000;
const TCP_LOCAL_PORT_STARTER: u16 = 49152;

pub struct NetworkStack {
    iface: Option<Interface>,
    device: NetStack,
    sockets: SocketSet<'static>,

    dhcp_handle: SocketHandle,
    dns_handle: SocketHandle,
    icmp_handle: SocketHandle,
    tcp_handle: SocketHandle,

    ip: Option<Ipv4Addr>,
    gateway: Option<Ipv4Addr>,
    dns: Option<Ipv4Addr>,

    dhcp_configured: bool,
    dns_state: DnsState,
    ping_state: PingState,
    tcp_state: TcpState,

    pending_events: Deque<NetEvent, 8>,

    ping_seq: u16,

    tcp_local_port: u16,
}

static mut SOCKET_STORAGE: [SocketStorage; 4] = [SocketStorage::EMPTY; 4];
static mut DNS_QUERIES: [Option<dns::DnsQuery>; 1] = [None];
static mut TCP_RX_BUF: [u8; 1024] = [0; 1024];
static mut TCP_TX_BUF: [u8; 1024] = [0; 1024];

impl NetworkStack {
    pub fn new() -> Self {
        let device = NetStack::new(&NETDEV);

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

        // TCP socket
        let tcp_socket = TcpSocket::new(
            tcp::SocketBuffer::new(unsafe { &mut TCP_RX_BUF[..] }),
            tcp::SocketBuffer::new(unsafe { &mut TCP_TX_BUF[..] }),
        );

        let tcp_handle = sockets.add(tcp_socket);

        Self {
            iface: None,
            device,
            sockets,
            dhcp_handle,
            dns_handle,
            icmp_handle,
            tcp_handle,
            ip: None,
            gateway: None,
            dns: None,
            dhcp_configured: false,
            dns_state: DnsState::Idle,
            ping_state: PingState::Idle,
            tcp_state: TcpState::Idle,
            pending_events: Deque::new(),
            ping_seq: 0,
            tcp_local_port: TCP_LOCAL_PORT_STARTER,
        }
    }

    pub fn config(&mut self, mac: EthernetAddress, seed: u64) {
        let mut config = Config::new(mac.into());

        config.random_seed = seed;

        self.iface = Some(Interface::new(
            config,
            &mut self.device,
            Instant::from_millis(syscall::get_tick() as i64),
        ));
    }

    fn push_event(&mut self, event: NetEvent) {
        if self.pending_events.push_back(event).is_err() {
            defmt::warn!("NETSRV: event queue full");
        }
    }

    pub fn take_event(&mut self) -> Option<NetEvent> {
        self.pending_events.pop_front()
    }

    pub fn reset(&mut self) {
        self.iface = None;
        self.ip = None;
        self.gateway = None;
        self.dns = None;
        self.dhcp_configured = false;
        self.dns_state = DnsState::Idle;
        self.ping_state = PingState::Idle;
        self.tcp_state = TcpState::Idle;
        self.pending_events.clear();
        self.reset_tcp_socket();

        self.sockets
            .get_mut::<DnsSocket>(self.dns_handle)
            .update_servers(&[]);
    }

    pub fn poll(&mut self) {
        self.iface_poll_once();

        self.dhcp_poll();
        self.dns_poll();
        self.icmp_poll();
        self.tcp_poll();

        self.iface_poll_once();

        self.poll_state_events();
    }

    fn iface_poll_once(&mut self) {
        if let Some(iface) = self.iface.as_mut() {
            let now = Instant::from_millis(syscall::get_tick() as i64);
            let _ = iface.poll(now, &mut self.device, &mut self.sockets);
        }
    }

    fn dhcp_poll(&mut self) {
        let event = {
            let dhcp = self.sockets.get_mut::<Dhcpv4Socket>(self.dhcp_handle);

            match dhcp.poll() {
                Some(Dhcpv4Event::Configured(config)) => {
                    Some(NetEvent::DhcpConfigured(Ipv4Config {
                        address: config.address.address(),
                        prefix_len: config.address.prefix_len(),
                        gateway: config.router,
                        dns: config.dns_servers.first().copied(),
                    }))
                }

                Some(Dhcpv4Event::Deconfigured) => {
                    if self.dhcp_configured {
                        Some(NetEvent::DhcpDeconfigured)
                    } else {
                        defmt::info!("NETSRV: ignore initial DHCP Deconfigured");
                        None
                    }
                }

                None => None,
            }
        };

        match event {
            Some(NetEvent::DhcpConfigured(config)) => {
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

                let socket = self.sockets.get_mut::<DnsSocket>(self.dns_handle);

                match config.dns {
                    Some(server) => {
                        socket.update_servers(&[IpAddress::Ipv4(server)]);
                    }
                    None => {
                        socket.update_servers(&[]);
                        defmt::warn!("NETSRV: DHCP did not provide DNS server");
                    }
                }

                if let Some(iface) = self.iface.as_mut() {
                    iface.update_ip_addrs(|addrs| {
                        addrs.clear();

                        let cidr = Ipv4Cidr::new(config.address, config.prefix_len);

                        if addrs.push(IpCidr::Ipv4(cidr)).is_err() {
                            defmt::warn!("NETSRV: failed to add interface address");
                        }
                    });

                    let _ = iface.routes_mut().remove_default_ipv4_route();

                    match config.gateway {
                        Some(gateway) => {
                            if iface.routes_mut().add_default_ipv4_route(gateway).is_err() {
                                defmt::warn!("NETSRV: failed to add default gateway");
                            }
                        }

                        None => {
                            defmt::warn!("NETSRV: DHCP did not provide gateway");
                        }
                    }
                }

                defmt::info!("NETSRV: DHCP configured");

                self.push_event(NetEvent::DhcpConfigured(config));
            }

            Some(NetEvent::DhcpDeconfigured) => {
                defmt::warn!("NETSRV: DHCP deconfigured");

                self.cancel_pending_dns(DnsEvent::Failed);
                self.cancel_pending_ping();
                self.cancel_pending_tcp();

                self.dhcp_configured = false;
                self.ip = None;
                self.gateway = None;
                self.dns = None;

                self.sockets
                    .get_mut::<DnsSocket>(self.dns_handle)
                    .update_servers(&[]);

                if let Some(iface) = self.iface.as_mut() {
                    iface.update_ip_addrs(|addrs| addrs.clear());

                    let _ = iface.routes_mut().remove_default_ipv4_route();
                }

                self.push_event(NetEvent::DhcpDeconfigured);
            }

            _ => {}
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

        let result = {
            let socket = self.sockets.get_mut::<DnsSocket>(self.dns_handle);
            socket.get_query_result(query)
        };

        match result {
            Ok(addrs) => {
                let ipv4_addr = addrs.iter().find_map(|addr| match addr {
                    IpAddress::Ipv4(addr) => Some(*addr),
                });

                self.dns_state = match ipv4_addr {
                    Some(addr) => {
                        defmt::info!(
                            "NETSRV: DNS resolved: {}.{}.{}.{}",
                            addr.octets()[0],
                            addr.octets()[1],
                            addr.octets()[2],
                            addr.octets()[3],
                        );

                        DnsState::Done(DnsEvent::Resolved { addr })
                    }

                    None => {
                        defmt::warn!("NETSRV: DNS query completed without IPv4 address");

                        DnsState::Done(DnsEvent::Failed)
                    }
                }
            }

            Err(GetQueryResultError::Pending) => {
                let elapsed_ms = syscall::get_tick().wrapping_sub(started_tick);

                if elapsed_ms < DNS_TIMEOUT_MS {
                    return;
                }

                defmt::warn!("NETSRV: DNS query timeout");

                {
                    let socket = self.sockets.get_mut::<DnsSocket>(self.dns_handle);
                    socket.cancel_query(query);
                }

                self.dns_state = DnsState::Done(DnsEvent::Timeout);
            }

            Err(GetQueryResultError::Failed) => {
                defmt::warn!("NETSRV: DNS query failed");

                self.dns_state = DnsState::Done(DnsEvent::Failed);
            }
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
                defmt::warn!("NETSRV: invalid ICMP packet");
                continue;
            };

            let Ok(repr) = Icmpv4Repr::parse(&packet, &ChecksumCapabilities::default()) else {
                defmt::warn!("NETSRV: ICMP parse failed");
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

    fn tcp_poll(&mut self) {
        let state = self.tcp_state;

        match state {
            TcpState::Idle | TcpState::Done(_) => {}

            TcpState::Connecting {
                target,
                port,
                data,
                started_tick,
            } => {
                let elapsed_ms = syscall::get_tick().wrapping_sub(started_tick);

                if elapsed_ms >= TCP_CONNECT_TIMEOUT_MS {
                    self.reset_tcp_socket();

                    self.tcp_state = TcpState::Done(TcpEvent::Timeout { addr: target, port });

                    return;
                }

                let (connected, active) = {
                    let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);
                    (socket.may_send(), socket.is_active())
                };

                if connected {
                    self.tcp_state = TcpState::Sending {
                        target,
                        port,
                        data,
                        sent: 0,
                        started_tick,
                    };

                    return;
                }

                if !active {
                    self.reset_tcp_socket();

                    self.tcp_state = TcpState::Done(TcpEvent::ConnectFailed { addr: target, port });
                }
            }

            TcpState::Sending {
                target,
                port,
                data,
                sent,
                started_tick,
            } => {
                let mut new_sent = sent;

                {
                    let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

                    if socket.can_send() && sent < data.len {
                        match socket.send_slice(&data.buf[sent..data.len]) {
                            Ok(count) => new_sent += count,
                            Err(_) => {
                                socket.abort();
                                self.tcp_state =
                                    TcpState::Done(TcpEvent::Closed { addr: target, port });

                                return;
                            }
                        }
                    }
                }

                if new_sent >= data.len {
                    self.tcp_state = TcpState::Receiving {
                        target,
                        port,
                        expected: data,
                        received: FixedStr {
                            buf: [0; 128],
                            len: 0,
                        },
                        started_tick,
                    };
                } else {
                    self.tcp_state = TcpState::Sending {
                        target,
                        port,
                        data,
                        sent: new_sent,
                        started_tick,
                    };
                }
            }

            TcpState::Receiving {
                target,
                port,
                expected,
                mut received,
                started_tick,
            } => {
                {
                    let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

                    while socket.can_recv() && received.len < received.buf.len() {
                        let remaining = &mut received.buf[received.len..];

                        match socket.recv_slice(remaining) {
                            Ok(0) => break,
                            Ok(count) => received.len += count,
                            Err(_) => break,
                        }
                    }
                }

                if received.len >= expected.len {
                    let matches = received.buf[..expected.len] == expected.buf[..expected.len];

                    let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);
                    socket.close();

                    if matches {
                        self.tcp_state = TcpState::Done(TcpEvent::EchoReply {
                            addr: target,
                            port,
                            data: received,
                            elapsed_ms: syscall::get_tick().wrapping_sub(started_tick),
                        });
                    } else {
                        self.tcp_state = TcpState::Done(TcpEvent::Closed { addr: target, port });
                    }

                    return;
                }

                let active = {
                    let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);
                    socket.is_active()
                };

                if !active {
                    self.abort_tcp_socket();

                    self.tcp_state = TcpState::Done(TcpEvent::Closed { addr: target, port });

                    return;
                }

                if syscall::get_tick().wrapping_sub(started_tick) >= TCP_ECHO_TIMEOUT_MS {
                    self.abort_tcp_socket();

                    self.tcp_state = TcpState::Done(TcpEvent::Timeout { addr: target, port });

                    return;
                }

                self.tcp_state = TcpState::Receiving {
                    target,
                    port,
                    expected,
                    received,
                    started_tick,
                };
            }
        }
    }

    fn cancel_pending_dns(&mut self, event: DnsEvent) {
        let query = match &self.dns_state {
            DnsState::Waiting { query, .. } => Some(*query),
            _ => None,
        };

        let Some(query) = query else { return };

        let socket = self.sockets.get_mut::<DnsSocket>(self.dns_handle);
        socket.cancel_query(query);

        self.dns_state = DnsState::Done(event);
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

    fn cancel_pending_tcp(&mut self) {
        let endpoint = match self.tcp_state {
            TcpState::Connecting { target, port, .. }
            | TcpState::Sending { target, port, .. }
            | TcpState::Receiving { target, port, .. } => Some((target, port)),
            TcpState::Idle | TcpState::Done(_) => None,
        };

        let Some((target, port)) = endpoint else {
            return;
        };

        {
            let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

            match self.tcp_state {
                TcpState::Connecting { .. } => socket.close(),
                TcpState::Sending { .. } | TcpState::Receiving { .. } => socket.abort(),
                TcpState::Idle | TcpState::Done(_) => {}
            }
        }

        self.tcp_state = TcpState::Done(TcpEvent::NetworkDown { addr: target, port });
    }

    fn abort_tcp_socket(&mut self) {
        let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

        if socket.is_open() {
            socket.abort();
        }
    }

    fn reset_tcp_socket(&mut self) {
        let old_socket = self.sockets.remove(self.tcp_handle);
        drop(old_socket);

        let socket = TcpSocket::new(
            tcp::SocketBuffer::new(unsafe { &mut TCP_RX_BUF[..] }),
            tcp::SocketBuffer::new(unsafe { &mut TCP_TX_BUF[..] }),
        );

        self.tcp_handle = self.sockets.add(socket);
    }

    fn poll_state_events(&mut self) {
        if let Some(event) = self.take_dns_event() {
            self.push_event(NetEvent::Dns(event));
        }

        if let Some(event) = self.take_ping_event() {
            self.push_event(NetEvent::Ping(event));
        }

        if let Some(event) = self.take_tcp_event() {
            self.push_event(NetEvent::Tcp(event));
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

    fn take_ping_event(&mut self) -> Option<PingEvent> {
        match self.ping_state {
            PingState::Done(event) => {
                self.ping_state = PingState::Idle;
                Some(event)
            }
            _ => None,
        }
    }

    fn take_tcp_event(&mut self) -> Option<TcpEvent> {
        match self.tcp_state {
            TcpState::Done(event) => {
                self.tcp_state = TcpState::Idle;
                Some(event)
            }
            _ => None,
        }
    }

    pub fn resolve(&mut self, hostname: &str) -> Result<(), NetworkError> {
        if !matches!(self.dns_state, DnsState::Idle) {
            return Err(NetworkError::Busy);
        }

        if self.dns.is_none() {
            return Err(NetworkError::NoDnsServer);
        }

        if hostname.is_empty() {
            return Err(NetworkError::InvalidArgument);
        }

        let iface = self.iface.as_mut().ok_or(NetworkError::NotReady)?;

        let socket = self.sockets.get_mut::<dns::Socket>(self.dns_handle);

        let query = socket
            .start_query(iface.context(), hostname, smoltcp::wire::DnsQueryType::A)
            .map_err(|e| {
                defmt::warn!("NETSRV: DNS query start failed: {}", e as i32);
                NetworkError::Dns(e)
            })?;

        self.dns_state = DnsState::Waiting {
            query,
            started_tick: syscall::get_tick(),
        };

        Ok(())
    }

    pub fn ping(&mut self, target: Ipv4Addr) -> Result<(), NetworkError> {
        if !matches!(self.ping_state, PingState::Idle) {
            return Err(NetworkError::Busy);
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
                defmt::warn!("NETSRV: ICMP bind failed");
                NetworkError::IcmpBindFailed
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
                defmt::warn!("NETSRV: ICMP send failed");
                NetworkError::IcmpSendFailed
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

    pub fn tcp_echo(
        &mut self,
        target: Ipv4Addr,
        port: u16,
        data: FixedStr<128>,
    ) -> Result<(), NetworkError> {
        if !matches!(self.tcp_state, TcpState::Idle) {
            return Err(NetworkError::Busy);
        }

        if port == 0 || data.len == 0 {
            return Err(NetworkError::InvalidArgument);
        }

        if !self.dhcp_configured || self.ip.is_none() {
            self.tcp_state = TcpState::Done(TcpEvent::NetworkDown { addr: target, port });

            return Ok(());
        }

        let socket_open = {
            let socket = self.sockets.get::<tcp::Socket>(self.tcp_handle);
            socket.is_open()
        };

        if socket_open {
            self.reset_tcp_socket();
        }

        let local_port = self.tcp_local_port;

        self.tcp_local_port = if self.tcp_local_port == u16::MAX {
            TCP_LOCAL_PORT_STARTER
        } else {
            self.tcp_local_port + 1
        };

        {
            let iface = self.iface.as_mut().ok_or(NetworkError::NotReady)?;

            let socket = self.sockets.get_mut::<tcp::Socket>(self.tcp_handle);

            socket
                .connect(iface.context(), (IpAddress::Ipv4(target), port), local_port)
                .map_err(|_| {
                    defmt::warn!("NETSRV: TCP connect start failed");
                    NetworkError::InvalidState
                })?;
        }

        self.tcp_state = TcpState::Connecting {
            target,
            port,
            data,
            started_tick: syscall::get_tick(),
        };

        Ok(())
    }
}
