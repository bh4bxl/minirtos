use core::{sync::atomic::AtomicBool, sync::atomic::Ordering};

use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::{
        dhcpv4::{Event as Dhcpv4Event, Socket as Dhcpv4Socket},
        icmp,
    },
    time::Instant,
    wire::{EthernetAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr},
};

use crate::{
    drivers::wlan::cyw43::cyw43_country::*,
    net::{
        self, WifiAuth, WifiState, WlanPollResult, fake_device::FakeNetDevice,
        smol_device::SmolDevice, wlan,
    },
    sys::{
        device_driver::{self, DeviceIrq, DeviceIrqEvent},
        sync::{event::Event, message_queue::MessageQueue},
        syscall::{self, sleep_ms},
        task::{Priority, TaskStack},
    },
};

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

#[derive(Clone, Copy)]
pub enum WlanCmd {
    Scan,
    Connect {
        ssid: FixedStr<32>,
        password: Option<FixedStr<64>>,
        auth: WifiAuth,
    },
    Disconnect,
    Ping,
}

pub static WLAN_CMD_QUEUE: MessageQueue<WlanCmd, 4> = MessageQueue::new();
pub static WLAN_SCAN_DONE: Event = Event::new(false);
pub static WLAN_CONNECT_DONE: Event = Event::new(false);
pub static WLAN_DISCONNECT_DONE: Event = Event::new(false);

const WLAN_PRIO: u8 = 150;

const WLAN_SIZE: usize = 4096;
static WLAN_STACK: TaskStack<WLAN_SIZE> = TaskStack::new();

pub fn start_wlan() -> Result<(), &'static str> {
    if let Err(x) = syscall::thread_create(
        wlan_task_entry,
        core::ptr::null_mut(),
        WLAN_STACK.get(),
        Priority(WLAN_PRIO),
        "wlan",
    ) {
        return Err(x);
    }

    Ok(())
}

static mut ICMP_RX_META: [icmp::PacketMetadata; 4] = [icmp::PacketMetadata::EMPTY; 4];
static mut ICMP_TX_META: [icmp::PacketMetadata; 4] = [icmp::PacketMetadata::EMPTY; 4];
static mut ICMP_RX_BUF: [u8; 256] = [0; 256];
static mut ICMP_TX_BUF: [u8; 256] = [0; 256];

static GPIO15_PENDING: AtomicBool = AtomicBool::new(false);

static NETDEV: FakeNetDevice = FakeNetDevice::new();

/// Thread entry
extern "C" fn wlan_task_entry(_arg: *mut ()) -> ! {
    net::wlan().wifi_on(CYW43_COUNTRY_CANADA, None).unwrap();

    let mac_addr = wlan().get_mac_addr().unwrap();
    defmt::info!(
        "WLAN: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac_addr[0],
        mac_addr[1],
        mac_addr[2],
        mac_addr[3],
        mac_addr[4],
        mac_addr[5]
    );

    let gpio = match device_driver::driver_manager().open_device(device_driver::DeviceType::Gpio, 0)
    {
        Some(dev) => dev,
        None => loop {
            defmt::warn!("No uart device found");
            cortex_m::asm::wfi();
        },
    };
    gpio.set_irq_callback(Some(gpio_irq_callback)).ok();
    let mut level = true;

    let hw_addr = EthernetAddress(mac_addr);
    let mut config = Config::new(hw_addr.into());
    config.random_seed = 0x1234_5678;

    let mut dev = SmolDevice::new(&NETDEV);

    let now = Instant::from_millis(0);
    let mut iface = Interface::new(config, &mut dev, now);

    let mut sockets_storage = [smoltcp::iface::SocketStorage::EMPTY; 4];
    let mut sockets = SocketSet::new(&mut sockets_storage[..]);

    let dhcp_handle = sockets.add(Dhcpv4Socket::new());

    let mut rx_buf = [0u8; 1536];

    let mut last_state = WifiState::Down;

    let mut ip = [0u8; 4];

    let mut gateway = None;

    let icmp_socket = icmp::Socket::new(
        icmp::PacketBuffer::new(unsafe { &mut ICMP_RX_META[..] }, unsafe {
            &mut ICMP_RX_BUF[..]
        }),
        icmp::PacketBuffer::new(unsafe { &mut ICMP_TX_META[..] }, unsafe {
            &mut ICMP_TX_BUF[..]
        }),
    );

    let icmp_handle = sockets.add(icmp_socket);

    loop {
        // WAN RX/TX/Event
        match wlan().poll() {
            Ok(WlanPollResult::Rx) => match wlan().get_rx_buf(&mut rx_buf) {
                Ok(len) => {
                    defmt::info!("WLAN RX frame len={}", len);

                    NETDEV.inject_rx(&rx_buf[..len]);
                }

                Err(e) => {
                    defmt::warn!("get_rx_buf failed: {:?}", e as usize);
                }
            },

            Ok(WlanPollResult::None) => {}

            Err(e) => {
                defmt::warn!("poll failed: {}", e as usize);
            }
        }

        // smoltcp poll
        let now = Instant::from_millis(syscall::get_tick() as i64);

        if last_state == WifiState::Connected {
            let _ = iface.poll(now, &mut dev, &mut sockets);

            {
                let socket = sockets.get_mut::<icmp::Socket>(icmp_handle);

                while socket.can_recv() {
                    match socket.recv() {
                        Ok((data, _endpoint)) => {
                            defmt::info!("ICMP reply recv len={}", data.len());
                        }

                        Err(_) => {
                            defmt::warn!("ICMP recv failed");
                            break;
                        }
                    }
                }
            }

            let dhcp = sockets.get_mut::<Dhcpv4Socket>(dhcp_handle);

            if let Some(event) = dhcp.poll() {
                match event {
                    Dhcpv4Event::Configured(config) => {
                        ip.copy_from_slice(&config.address.address().octets());
                        defmt::info!(
                            "DHCP configured, IP={}.{}.{}.{}",
                            ip[0],
                            ip[1],
                            ip[2],
                            ip[3],
                        );

                        gateway = config.router;

                        iface.update_ip_addrs(|addrs| {
                            addrs.clear();

                            let _ = addrs.push(IpCidr::Ipv4(config.address));
                        });
                    }

                    Dhcpv4Event::Deconfigured => {
                        defmt::warn!("DHCP deconfigured");

                        iface.update_ip_addrs(|addrs| {
                            addrs.clear();
                        });
                    }
                }
            }
        }

        while let Some(handle) = NETDEV.take_tx() {
            let sent = NETDEV.with_packet(handle, |pkt| {
                let ethertype = u16::from_be_bytes([pkt[12], pkt[13]]);
                defmt::info!(
                    "WLAN TX frame len={} ethertype=0x{:04x}",
                    pkt.len(),
                    ethertype
                );
                wlan().sent_tx_buf(pkt)
            });

            match sent {
                Some(Ok(())) => {}
                Some(Err(e)) => {
                    defmt::warn!("WLAN send_data failed: {:?}", e as usize);
                }
                None => {
                    defmt::warn!("WLAN TX packet missing");
                }
            }

            NETDEV.free_packet(handle);
        }

        // Shell command
        if let Some(cmd) = WLAN_CMD_QUEUE.try_recv() {
            match cmd {
                WlanCmd::Scan => {
                    defmt::info!("wifi scan requested");

                    if net::wlan().wifi_scan().is_ok() {
                        loop {
                            let _ = net::wlan().poll();

                            if net::wlan().wifi_scan_done().unwrap() {
                                break;
                            }

                            sleep_ms(10);
                        }

                        let mut res = heapless::Vec::new();
                        net::wlan().wifi_scan_results(&mut res).unwrap();

                        res.iter().for_each(|r| {
                            crate::println!(
                                "[{:>3} dBm] ch={:<3} {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}  {}",
                                r.rssi,
                                r.channel,
                                r.bssid[0],
                                r.bssid[1],
                                r.bssid[2],
                                r.bssid[3],
                                r.bssid[4],
                                r.bssid[5],
                                if r.ssid_len > 0 {
                                    core::str::from_utf8(&r.ssid).ok().unwrap()
                                } else {
                                    "<Hidden SSID>"
                                },
                            );
                        });
                        crate::println!("Total: {}", res.len());
                    } else {
                        defmt::warn!("wifi scan start failed");
                    }

                    WLAN_SCAN_DONE.signal();
                }
                WlanCmd::Connect {
                    ssid,
                    password,
                    auth,
                } => {
                    defmt::info!("wifi connect requested");

                    let password_str = match &password {
                        Some(password) => password.as_str(),

                        None => "",
                    };

                    if wlan()
                        .wifi_connect(ssid.as_str(), password_str, auth)
                        .is_err()
                    {
                        defmt::warn!("wifi connect failed");
                    }
                }
                WlanCmd::Disconnect => {
                    defmt::info!("wifi disconnect requested");
                    wlan().wifi_disconnect().ok();
                    loop {
                        let _ = net::wlan().poll();

                        if net::wlan().wifi_status().unwrap() == net::WifiState::Down {
                            WLAN_DISCONNECT_DONE.signal();
                            break;
                        }

                        sleep_ms(10);
                    }
                }
                WlanCmd::Ping => {
                    defmt::info!("ping gateway request");
                    if let Some(gw) = gateway {
                        let socket = sockets.get_mut::<icmp::Socket>(icmp_handle);
                        if !socket.is_open() {
                            socket.bind(icmp::Endpoint::Ident(0x1234)).ok();
                        }
                        let echo = Icmpv4Repr::EchoRequest {
                            ident: 0x1234,
                            seq_no: 1,
                            data: b"miniRTOS",
                        };
                        let payload_len = echo.buffer_len();
                        let mut buf = socket.send(payload_len, IpAddress::Ipv4(gw)).unwrap();

                        let mut packet = Icmpv4Packet::new_unchecked(&mut buf);

                        echo.emit(&mut packet, &smoltcp::phy::ChecksumCapabilities::default());
                        defmt::info!("ICMP echo request sent");
                    } else {
                        defmt::warn!("no gateway");
                    }
                }
            }
        }

        let state = wlan().wifi_status().unwrap();
        if state != last_state {
            if state == WifiState::Connected {
                WLAN_CONNECT_DONE.signal();
            }

            if state == WifiState::Down {
                WLAN_DISCONNECT_DONE.signal();
            }

            last_state = state;
        }

        // Key
        if GPIO15_PENDING.swap(false, Ordering::AcqRel) {
            defmt::info!("GPIO15 triggered @{}", syscall::get_tick());

            if net::wlan().wifi_gpio_ctrl(0, level).is_ok() {
                level = !level;
            }
        }

        sleep_ms(10);
    }
}

fn gpio_irq_callback(irq: DeviceIrq) {
    if irq.event != DeviceIrqEvent::Gpio {
        return;
    }

    if irq.data & 0xff == 15 && irq.data & 0xff00 == 0 {
        GPIO15_PENDING.store(true, Ordering::Release);
    }
}
