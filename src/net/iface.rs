use smoltcp::{
    iface::{Config, Interface, SocketSet, SocketStorage},
    time::Instant,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr},
};

use crate::net::smol_device::SmolDevice;

static mut SOCKET_STORAGE: [SocketStorage; 4] = [SocketStorage::EMPTY; 4];

pub struct NetIf {
    iface: Interface,
    sockets: SocketSet<'static>,
    device: SmolDevice,
    time_ms: i64,
}

impl NetIf {
    pub fn new(device: SmolDevice) -> Self {
        let hw_addr =
            HardwareAddress::Ethernet(EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]));

        let config = Config::new(hw_addr);

        let mut device = device;

        let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));

        iface.update_ip_addrs(|addrs| {
            let _ = addrs.push(IpCidr::new(IpAddress::v4(192, 168, 1, 10), 24));
        });

        let sockets = SocketSet::new(unsafe { &mut SOCKET_STORAGE[..] });

        Self {
            iface,
            sockets,
            device,
            time_ms: 0,
        }
    }

    pub fn poll(&mut self) {
        let timestamp = Instant::from_millis(self.time_ms);

        let _ = self
            .iface
            .poll(timestamp, &mut self.device, &mut self.sockets);

        self.time_ms += 10;
    }

    pub fn sockets(&mut self) -> &mut SocketSet<'static> {
        &mut self.sockets
    }
}
