use core::net::Ipv4Addr;

use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::udp;
use smoltcp::time::Instant;
#[allow(unused_imports)]
use smoltcp::wire::{
    EthernetAddress, EthernetFrame, EthernetProtocol, IpAddress, IpCidr, IpProtocol, Ipv4Packet,
    UdpPacket,
};

use crate::net::fake_device::FakeNetDevice;
use crate::net::iface::NetIf;
use crate::net::smol_device::SmolDevice;
use crate::net::{ethernet, sockets};
use crate::sys::{
    syscall,
    task::{Priority, TaskStack},
};

const NET_TASK_PRIORTIY: u8 = 100;

const NET_TASK_STACK_SIZE: usize = 1024;
#[allow(dead_code)]
static SEND_STACK: TaskStack<NET_TASK_STACK_SIZE> = TaskStack::new();
#[allow(dead_code)]
static RECV_STACK: TaskStack<NET_TASK_STACK_SIZE> = TaskStack::new();
static UDP_STACK: TaskStack<NET_TASK_STACK_SIZE> = TaskStack::new();

pub fn start_net_test_apps() -> Result<(), &'static str> {
    // let Ok(_) = syscall::thread_create(
    //     inject_entry,
    //     core::ptr::null_mut(),
    //     SEND_STACK.get(),
    //     Priority(NET_TASK_PRIORTIY),
    //     "net_send",
    // ) else {
    //     return Err("Failed to create task1");
    // };

    // let Ok(_) = syscall::thread_create(
    //     smoltcp_entry,
    //     core::ptr::null_mut(),
    //     RECV_STACK.get(),
    //     Priority(NET_TASK_PRIORTIY),
    //     "net_recv",
    // ) else {
    //     return Err("Failed to create task1");
    // };

    let Ok(_) = syscall::thread_create(
        udp_echo_task,
        core::ptr::null_mut(),
        UDP_STACK.get(),
        Priority(NET_TASK_PRIORTIY),
        "net_recv",
    ) else {
        return Err("Failed to create task1");
    };

    Ok(())
}

static NETDEV: FakeNetDevice = FakeNetDevice::new();

#[allow(dead_code)]
extern "C" fn inject_entry(_: *mut ()) -> ! {
    loop {
        syscall::sleep_ms(1000);

        let arp_request: [u8; 42] = [
            // Ethernet header
            // dst MAC: broadcast
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // src MAC: sender
            0x02, 0x00, 0x00, 0x00, 0x00, 0x02, // ethertype: ARP
            0x08, 0x06, // ARP payload
            // hardware type: Ethernet
            0x00, 0x01, // protocol type: IPv4
            0x08, 0x00, // hardware size: 6
            0x06, // protocol size: 4
            0x04, // opcode: request
            0x00, 0x01, // sender MAC
            0x02, 0x00, 0x00, 0x00, 0x00, 0x02, // sender IP: 192.168.1.10
            192, 168, 1, 10, // target MAC: unknown
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // target IP: 192.168.1.100
            192, 168, 1, 100,
        ];

        let _ = NETDEV.inject_rx(&arp_request);
        defmt::info!("inject ARP request");

        syscall::sleep_ms(100);

        // IPv4 + ICMP Echo Request
        let icmp_request: [u8; 60] = [
            // Ethernet header
            0x02, 0x00, 0x00, 0x00, 0x00, 0x01, // dst MAC: device
            0x02, 0x00, 0x00, 0x00, 0x00, 0x02, // src MAC: fake host
            0x08, 0x00, // IPv4
            // IPv4 header
            0x45, 0x00, // version/IHL, DSCP
            0x00, 0x1c, // total length = 28
            0x12, 0x34, // identification
            0x00, 0x00, // flags/fragment offset
            64,   // TTL
            1,    // protocol = ICMP
            0xe4, 0xee, // IPv4 header checksum
            192, 168, 1, 10, // src IP
            192, 168, 1, 100, // dst IP
            // ICMP Echo Request
            8, // type = echo request
            0, // code
            0xf7, 0xfd, // ICMP checksum
            0x00, 0x01, // identifier
            0x00, 0x01, // sequence
            // padding to Ethernet minimum frame size
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let _ = NETDEV.inject_rx(&icmp_request);
        defmt::info!("inject ICMP echo request");
    }
}

#[allow(dead_code)]
extern "C" fn smoltcp_entry(_: *mut ()) -> ! {
    let hw_addr = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);

    let mut config = Config::new(hw_addr.into());
    config.random_seed = 0x1234_5678;

    let mut dev = SmolDevice::new(&NETDEV);

    let now = Instant::from_millis(0);
    let mut iface = Interface::new(config, &mut dev, now);

    iface.update_ip_addrs(|addrs| {
        match addrs.push(IpCidr::new(Ipv4Addr::new(192, 168, 1, 100).into(), 24)) {
            Ok(_) => defmt::info!("ip addr added"),
            Err(_) => defmt::warn!("ip addr add failed"),
        }
    });

    let mut sockets_storage = [];
    let mut sockets = SocketSet::new(&mut sockets_storage[..]);

    loop {
        NETDEV.wait_rx();

        let now = Instant::from_millis(syscall::get_tick() as i64);

        let _ = iface.poll(now, &mut dev, &mut sockets);

        while let Some(handle) = dev.take_tx() {
            NETDEV.with_packet(handle, |data| {
                defmt::info!("smoltcp tx frame len={}", data.len());

                if let Some(header) = ethernet::parse_ethernet_frame(data) {
                    defmt::info!("tx ethertype={=u16:#06x}", header.ethertype);
                }

                // ARP packet?
                if data.len() >= 42 && data[12] == 0x08 && data[13] == 0x06 {
                    let opcode = u16::from_be_bytes([data[20], data[21]]);

                    defmt::info!("tx ARP opcode={}", opcode);
                }

                if data.len() >= 42 && data[12] == 0x08 && data[13] == 0x00 {
                    let protocol = data[23];
                    defmt::info!("tx IPv4 protocol={}", protocol);

                    if protocol == 1 {
                        let icmp_type = data[34];
                        defmt::info!("tx ICMP type={}", icmp_type);
                    }
                }
            });

            NETDEV.free_packet(handle);
        }
    }
}

pub extern "C" fn udp_echo_task(_: *mut ()) -> ! {
    let smol_dev = SmolDevice::new(&NETDEV);

    let mut netif = NetIf::new(smol_dev);

    let udp_handle = sockets::create_udp_socket(netif.sockets());

    sockets::bind_udp(netif.sockets(), udp_handle, 1234);

    inject_udp_test_packet();

    loop {
        netif.poll();

        udp_echo_poll(netif.sockets(), udp_handle);

        dump_tx_packets();

        syscall::sleep_ms(1000);
    }
}

fn udp_echo_poll(sockets: &mut smoltcp::iface::SocketSet<'static>, handle: SocketHandle) {
    let socket = sockets.get_mut::<udp::Socket>(handle);

    while socket.can_recv() {
        let mut echo_buf = [0u8; 512];

        let recv = socket.recv();

        let (len, endpoint) = match recv {
            Ok((data, meta)) => {
                let len = core::cmp::min(data.len(), echo_buf.len());
                echo_buf[..len].copy_from_slice(&data[..len]);

                let endpoint = meta.endpoint;

                match endpoint.addr {
                    smoltcp::wire::IpAddress::Ipv4(addr) => {
                        let b = addr.octets();

                        defmt::info!(
                            "udp recv {} bytes from {}.{}.{}.{}:{}",
                            len,
                            b[0],
                            b[1],
                            b[2],
                            b[3],
                            endpoint.port,
                        );
                    }
                }

                (len, endpoint)
            }
            Err(_) => {
                defmt::warn!("udp recv failed");
                break;
            }
        };

        if let Err(_) = socket.send_slice(&echo_buf[..len], endpoint) {
            defmt::warn!("udp echo send failed");
        }
    }
}

fn inject_udp_test_packet() {
    let src_mac = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x02]);
    let dst_mac = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);

    let src_ip = Ipv4Addr::new(192, 168, 1, 2);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 10);

    let payload = b"hello udp";

    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let eth_len = 14 + ip_len;

    let mut frame_buf = [0u8; 128];

    {
        let mut eth = EthernetFrame::new_unchecked(&mut frame_buf[..eth_len]);
        eth.set_src_addr(src_mac);
        eth.set_dst_addr(dst_mac);
        eth.set_ethertype(EthernetProtocol::Ipv4);

        let mut ip = Ipv4Packet::new_unchecked(eth.payload_mut());
        ip.set_version(4);
        ip.set_header_len(20);
        ip.set_total_len(ip_len as u16);
        ip.set_hop_limit(64);
        ip.set_next_header(IpProtocol::Udp);
        ip.set_src_addr(src_ip);
        ip.set_dst_addr(dst_ip);
        ip.fill_checksum();

        let mut udp = UdpPacket::new_unchecked(ip.payload_mut());
        udp.set_src_port(5555);
        udp.set_dst_port(1234);
        udp.set_len(udp_len as u16);
        udp.payload_mut().copy_from_slice(payload);

        udp.set_checksum(0);
    }

    let ok = NETDEV.inject_rx(&frame_buf[..eth_len]);

    defmt::info!("inject udp test packet: {}", ok);
}

fn dump_tx_packets() {
    while let Some(handle) = NETDEV.take_tx() {
        let _ = NETDEV.with_packet(handle, |data| {
            defmt::info!("tx packet len {}", data.len());
        });

        NETDEV.free_packet(handle);
    }
}
