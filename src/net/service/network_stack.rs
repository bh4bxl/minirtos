use core::net::{Ipv4Addr, SocketAddrV4};

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

use super::super::{NET_BUFFER_SIZE, NetResult, core::NetStack, with_buffer, with_buffer_mut};

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

    tcp_generation: u16,
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
            tcp_generation: 0,
        }
    }

    fn validate_tcp_socket(&self, socket: SocketId) -> NetResult<()> {
        if socket.index() != 0 || socket.generation() != self.tcp_generation {
            return Err(NetError::InvalidSocket);
        }

        if matches!(self.tcp_state, TcpState::Idle) {
            return Err(NetError::InvalidSocket);
        }

        Ok(())
    }

    fn next_tcp_socket_id(&mut self) -> SocketId {
        self.tcp_generation = self.tcp_generation.wrapping_add(1);

        if self.tcp_generation == 0 {
            self.tcp_generation = 1;
        }

        SocketId::new(0, self.tcp_generation)
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
        let pending_request = match self.tcp_state {
            TcpState::Connecting { request, .. }
            | TcpState::Sending { request, .. }
            | TcpState::Receiving { request, .. }
            | TcpState::Closing { request, .. } => Some(request),

            TcpState::Idle | TcpState::Open { .. } => None,
        };

        if let Some(request) = pending_request {
            self.push_event(NetEvent::TcpError {
                request,
                error: NetError::NetworkDown,
            });
        }

        self.iface = None;
        self.ip = None;
        self.gateway = None;
        self.dns = None;
        self.dhcp_configured = false;
        self.dns_state = DnsState::Idle;
        self.ping_state = PingState::Idle;

        self.reset_tcp_socket();
        self.tcp_state = TcpState::Idle;

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
        let waiting = match self.ping_state {
            PingState::Waiting {
                request,
                target,
                seq,
                sent_tick,
                timeout_ms,
            } => Some((request, target, seq, sent_tick, timeout_ms)),

            PingState::Idle => None,
        };

        let Some((request, target, seq, sent_tick, timeout_ms)) = waiting else {
            return;
        };

        /*
         * Try to receive a matching ICMP echo reply.
         */
        let reply = {
            let socket = self.sockets.get_mut::<IcmpSocket>(self.icmp_handle);

            let mut reply = None;

            while socket.can_recv() {
                let Ok((data, endpoint)) = socket.recv() else {
                    break;
                };

                let IpAddress::Ipv4(source) = endpoint;

                let Ok(packet) = Icmpv4Packet::new_checked(data) else {
                    defmt::warn!("NETSTACK: invalid ICMP packet");
                    continue;
                };

                let Ok(repr) = Icmpv4Repr::parse(&packet, &ChecksumCapabilities::default()) else {
                    defmt::warn!("NETSTACK: ICMP parse failed");
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

                if ident != 0x1234 || seq_no != seq || source != target {
                    continue;
                }

                reply = Some((source, data.len()));
                break;
            }

            reply
        };

        if let Some((addr, bytes)) = reply {
            let rtt_ms = syscall::get_tick().wrapping_sub(sent_tick);

            self.ping_state = PingState::Idle;

            self.push_event(NetEvent::IcmpReply {
                request,
                addr,
                sequence: seq,
                bytes,
                rtt_ms,
            });

            return;
        }

        /*
         * Timeout.
         */
        if syscall::get_tick().wrapping_sub(sent_tick) >= timeout_ms {
            self.ping_state = PingState::Idle;

            self.push_event(NetEvent::IcmpError {
                request,
                error: NetError::TimedOut,
            });
        }
    }

    fn tcp_poll(&mut self) {
        match self.tcp_state {
            TcpState::Idle | TcpState::Open { .. } => {}

            TcpState::Connecting {
                request,
                socket,
                started_tick,
                timeout_ms,
            } => {
                let (connected, active) = {
                    let tcp = self.sockets.get::<TcpSocket>(self.tcp_handle);

                    (tcp.may_send(), tcp.is_active())
                };

                if connected {
                    self.tcp_state = TcpState::Open { socket };

                    self.push_event(NetEvent::TcpConnected { request });
                    return;
                }

                if !active {
                    self.reset_tcp_socket();
                    self.tcp_state = TcpState::Open { socket };

                    self.push_event(NetEvent::TcpError {
                        request,
                        error: NetError::ConnectionRefused,
                    });

                    return;
                }

                if syscall::get_tick().wrapping_sub(started_tick) >= timeout_ms {
                    self.reset_tcp_socket();
                    self.tcp_state = TcpState::Open { socket };

                    self.push_event(NetEvent::TcpError {
                        request,
                        error: NetError::TimedOut,
                    });
                }
            }

            TcpState::Sending {
                request,
                socket,
                buffer,
                len,
                sent,
                started_tick,
                timeout_ms,
            } => {
                let result = with_buffer(buffer, |data| {
                    let tcp = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

                    if !tcp.can_send() {
                        return Ok(0);
                    }

                    tcp.send_slice(&data[sent..len])
                        .map_err(|_| NetError::ConnectionReset)
                });

                match result {
                    Ok(Ok(count)) => {
                        let total = sent + count;

                        if total >= len {
                            self.tcp_state = TcpState::Open { socket };

                            self.push_event(NetEvent::TcpSent {
                                request,
                                len: total,
                            });
                        } else {
                            self.tcp_state = TcpState::Sending {
                                request,
                                socket,
                                buffer,
                                len,
                                sent: total,
                                started_tick,
                                timeout_ms,
                            };
                        }
                    }

                    Ok(Err(error)) | Err(error) => {
                        self.tcp_state = TcpState::Open { socket };

                        self.push_event(NetEvent::TcpError { request, error });
                    }
                }

                if matches!(self.tcp_state, TcpState::Sending { .. })
                    && syscall::get_tick().wrapping_sub(started_tick) >= timeout_ms
                {
                    self.tcp_state = TcpState::Open { socket };

                    self.push_event(NetEvent::TcpError {
                        request,
                        error: NetError::TimedOut,
                    });
                }
            }

            TcpState::Receiving {
                request,
                socket,
                buffer,
                max_len,
                started_tick,
                timeout_ms,
            } => {
                let result = with_buffer_mut(buffer, |data, stored_len| {
                    let tcp = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

                    if !tcp.can_recv() {
                        return Ok(0);
                    }

                    let capacity = max_len.min(data.len());

                    let count = tcp
                        .recv_slice(&mut data[..capacity])
                        .map_err(|_| NetError::ConnectionReset)?;

                    *stored_len = count;

                    Ok(count)
                });

                match result {
                    Ok(Ok(count)) if count > 0 => {
                        self.tcp_state = TcpState::Open { socket };

                        self.push_event(NetEvent::TcpReceived {
                            request,
                            len: count,
                        });

                        return;
                    }

                    Ok(Ok(_)) => {}

                    Ok(Err(error)) | Err(error) => {
                        self.tcp_state = TcpState::Open { socket };

                        self.push_event(NetEvent::TcpError { request, error });

                        return;
                    }
                }

                let active = self.sockets.get::<TcpSocket>(self.tcp_handle).is_active();

                if !active {
                    self.tcp_state = TcpState::Open { socket };

                    // recv == 0 means EOF.
                    self.push_event(NetEvent::TcpReceived { request, len: 0 });

                    return;
                }

                if syscall::get_tick().wrapping_sub(started_tick) >= timeout_ms {
                    self.tcp_state = TcpState::Open { socket };

                    self.push_event(NetEvent::TcpError {
                        request,
                        error: NetError::TimedOut,
                    });
                }
            }

            TcpState::Closing {
                request,
                socket: _,
                started_tick,
            } => {
                let open = self.sockets.get::<TcpSocket>(self.tcp_handle).is_open();

                if !open {
                    self.reset_tcp_socket();
                    self.tcp_state = TcpState::Idle;

                    self.push_event(NetEvent::TcpClosed { request });
                    return;
                }

                if syscall::get_tick().wrapping_sub(started_tick) >= 3000 {
                    self.reset_tcp_socket();
                    self.tcp_state = TcpState::Idle;

                    self.push_event(NetEvent::TcpClosed { request });
                }
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
        let request = match self.ping_state {
            PingState::Waiting { request, .. } => Some(request),
            PingState::Idle => None,
        };

        let Some(request) = request else {
            return;
        };

        self.ping_state = PingState::Idle;

        self.push_event(NetEvent::IcmpError {
            request,
            error: NetError::NetworkDown,
        });
    }

    fn cancel_pending_tcp(&mut self) {
        let pending_request = match self.tcp_state {
            TcpState::Connecting { request, .. }
            | TcpState::Sending { request, .. }
            | TcpState::Receiving { request, .. }
            | TcpState::Closing { request, .. } => Some(request),

            TcpState::Idle | TcpState::Open { .. } => None,
        };

        if !matches!(self.tcp_state, TcpState::Idle) {
            self.reset_tcp_socket();
            self.tcp_state = TcpState::Idle;
        }

        if let Some(request) = pending_request {
            self.push_event(NetEvent::TcpError {
                request,
                error: NetError::NetworkDown,
            });
        }
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

    pub fn icmp_echo(
        &mut self,
        request: RequestId,
        target: Ipv4Addr,
        timeout_ms: u64,
    ) -> NetResult<()> {
        if !matches!(self.ping_state, PingState::Idle) {
            return Err(NetError::Busy);
        }

        if !self.dhcp_configured || self.ip.is_none() {
            return Err(NetError::NetworkDown);
        }

        let seq = self.ping_seq.wrapping_add(1);

        let socket = self.sockets.get_mut::<IcmpSocket>(self.icmp_handle);

        if !socket.is_open() {
            socket.bind(IcmpEndpoint::Ident(0x1234)).map_err(|_| {
                defmt::warn!("NETSRV: ICMP bind failed");
                NetError::IcmpBindFailed
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
                NetError::IcmpSendFailed
            })?;

        let mut packet = Icmpv4Packet::new_unchecked(buf);

        echo.emit(&mut packet, &smoltcp::phy::ChecksumCapabilities::default());

        self.ping_seq = seq;

        self.ping_state = PingState::Waiting {
            request,
            target,
            seq,
            sent_tick: syscall::get_tick(),
            timeout_ms,
        };

        Ok(())
    }

    pub fn tcp_open(&mut self) -> NetResult<SocketId> {
        if !matches!(self.tcp_state, TcpState::Idle) {
            return Err(NetError::NoSocketAvailable);
        }

        self.reset_tcp_socket();

        let socket = self.next_tcp_socket_id();

        self.tcp_state = TcpState::Open { socket };

        Ok(socket)
    }

    pub fn tcp_connect(
        &mut self,
        request: RequestId,
        socket: SocketId,
        remote: SocketAddrV4,
        timeout_ms: u64,
    ) -> NetResult<()> {
        self.validate_tcp_socket(socket)?;

        if !matches!(self.tcp_state, TcpState::Open { .. }) {
            return Err(NetError::Busy);
        }

        if remote.port() == 0 {
            return Err(NetError::InvalidPort);
        }

        if !self.dhcp_configured || self.ip.is_none() {
            return Err(NetError::NetworkDown);
        }

        let local_port = self.tcp_local_port;

        self.tcp_local_port = if local_port == u16::MAX {
            TCP_LOCAL_PORT_STARTER
        } else {
            local_port.wrapping_add(1)
        };

        {
            let iface = self.iface.as_mut().ok_or(NetError::NotConfigured)?;

            let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

            socket
                .connect(
                    iface.context(),
                    (IpAddress::Ipv4(*remote.ip()), remote.port()),
                    local_port,
                )
                .map_err(|_| NetError::ConnectionRefused)?;
        }

        self.tcp_state = TcpState::Connecting {
            request,
            socket,
            started_tick: syscall::get_tick(),
            timeout_ms,
        };

        Ok(())
    }

    pub fn tcp_send(
        &mut self,
        request: RequestId,
        socket: SocketId,
        buffer: BufferId,
        len: usize,
        timeout_ms: u64,
    ) -> NetResult<()> {
        self.validate_tcp_socket(socket)?;

        if !matches!(self.tcp_state, TcpState::Open { .. }) {
            return Err(NetError::Busy);
        }

        let available = with_buffer(buffer, |data| data.len())?;

        if len > available {
            return Err(NetError::InvalidBuffer);
        }

        let connected = self.sockets.get::<TcpSocket>(self.tcp_handle).may_send();

        if !connected {
            return Err(NetError::NotConnected);
        }

        self.tcp_state = TcpState::Sending {
            request,
            socket,
            buffer,
            len,
            sent: 0,
            started_tick: syscall::get_tick(),
            timeout_ms,
        };

        Ok(())
    }

    pub fn tcp_recv(
        &mut self,
        request: RequestId,
        socket: SocketId,
        buffer: BufferId,
        max_len: usize,
        timeout_ms: u64,
    ) -> NetResult<()> {
        self.validate_tcp_socket(socket)?;

        if !matches!(self.tcp_state, TcpState::Open { .. }) {
            return Err(NetError::Busy);
        }

        {
            let socket = self.sockets.get::<TcpSocket>(self.tcp_handle);

            if !socket.may_recv() && !socket.is_active() {
                return Err(NetError::NotConnected);
            }
        }

        with_buffer_mut(buffer, |_data, len| {
            *len = 0;
        })?;

        self.tcp_state = TcpState::Receiving {
            request,
            socket: socket,
            buffer,
            max_len: max_len.min(NET_BUFFER_SIZE),
            started_tick: syscall::get_tick(),
            timeout_ms,
        };

        Ok(())
    }

    pub fn tcp_close(&mut self, request: RequestId, socket: SocketId) -> NetResult<()> {
        self.validate_tcp_socket(socket)?;

        if !matches!(self.tcp_state, TcpState::Open { .. }) {
            return Err(NetError::Busy);
        }

        {
            let socket = self.sockets.get_mut::<TcpSocket>(self.tcp_handle);

            socket.close();
        }

        self.tcp_state = TcpState::Closing {
            request,
            socket: socket,
            started_tick: syscall::get_tick(),
        };

        Ok(())
    }

    pub fn tcp_abort(&mut self, socket: SocketId) {
        if self.validate_tcp_socket(socket).is_err() {
            return;
        }

        self.reset_tcp_socket();
        self.tcp_state = TcpState::Idle;
    }
}
