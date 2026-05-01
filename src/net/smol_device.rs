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
    handle: PacketHandle,
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
        let result = self
            .dev
            .with_packet_storage_mut(self.handle, len, |buf| f(buf))
            .expect("failed to prepare TX packet buffer");

        if !self.dev.try_send(self.handle) {
            self.dev.free_packet(self.handle);
        }

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
        let tx_handle = self.dev.alloc_tx_packet()?;

        let rx_handle = match self.dev.try_recv() {
            Some(handle) => handle,
            None => {
                self.dev.free_packet(tx_handle);
                return None;
            }
        };

        Some((
            MiniRxToken {
                dev: self.dev,
                handle: rx_handle,
            },
            MiniTxToken {
                dev: self.dev,
                handle: tx_handle,
            },
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        let handle = self.dev.alloc_tx_packet()?;

        Some(MiniTxToken {
            dev: self.dev,
            handle,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1500;
        caps
    }
}
