use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    time::Instant,
};

use crate::net::{fake_device::FakeNetDevice, packet::PacketHandle};

pub struct MiniRxToken<'a> {
    dev: &'a FakeNetDevice,
    handle: PacketHandle,
}

pub struct MiniTxToken<'a> {
    dev: &'a FakeNetDevice,
}

impl<'a> RxToken for MiniRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        let result = self.dev.with_packet(self.handle, |data| f(data)).unwrap();

        self.dev.free_packet(self.handle);

        result
    }
}

impl<'a> TxToken for MiniTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let handle = self.dev.alloc_tx_packet().unwrap();

        let result = self
            .dev
            .with_packet_storage_mut(handle, len, |buf| f(buf))
            .unwrap();

        self.dev.send(handle);

        result
    }
}

pub struct SmolDevice {
    dev: &'static FakeNetDevice,
}

#[allow(dead_code)]
impl SmolDevice {
    pub const fn new(dev: &'static FakeNetDevice) -> Self {
        Self { dev }
    }

    pub fn take_tx(&self) -> Option<PacketHandle> {
        self.dev.take_tx()
    }

    pub fn with_packet<R>(&self, handle: PacketHandle, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        self.dev.with_packet(handle, f)
    }

    pub fn free_packet(&self, handle: PacketHandle) {
        self.dev.free_packet(handle);
    }
}

impl Device for SmolDevice {
    type RxToken<'a>
        = MiniRxToken<'a>
    where
        Self: 'a;
    type TxToken<'a>
        = MiniTxToken<'a>
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let handle = self.dev.try_recv()?;

        Some((
            MiniRxToken {
                dev: self.dev,
                handle,
            },
            MiniTxToken { dev: self.dev },
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(MiniTxToken { dev: self.dev })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1500;
        caps
    }
}
