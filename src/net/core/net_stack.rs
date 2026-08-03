use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    time::Instant,
};

use super::{net_device::NetDevice, packet::PacketHandle};

pub struct MiniRxToken<'a> {
    dev: &'a NetDevice,
    handle: Option<PacketHandle>,
}

pub struct MiniTxToken<'a> {
    dev: &'a NetDevice,
    handle: Option<PacketHandle>,
}

impl<'a> RxToken for MiniRxToken<'a> {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        let handle = self.handle.take().unwrap();

        let result = self
            .dev
            .with_packet(handle, |data| f(data))
            .expect("RX packet missing");

        self.dev.free_packet(handle);

        result
    }
}

impl Drop for MiniRxToken<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.dev.free_packet(handle);
        }
    }
}

impl<'a> TxToken for MiniTxToken<'a> {
    fn consume<R, F>(mut self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let handle = self.handle.take().unwrap();

        let result = self
            .dev
            .with_packet_storage_mut(handle, len, |buf| f(buf))
            .expect("failed to prepare TX packet buffer");

        if !self.dev.try_send(handle) {
            self.dev.free_packet(handle);
        }

        result
    }
}

impl Drop for MiniTxToken<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.dev.free_packet(handle);
        }
    }
}

pub struct NetStack {
    dev: &'static NetDevice,
}

#[allow(dead_code)]
impl NetStack {
    pub const fn new(dev: &'static NetDevice) -> Self {
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

impl Device for NetStack {
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
                handle: Some(rx_handle),
            },
            MiniTxToken {
                dev: self.dev,
                handle: Some(tx_handle),
            },
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        let handle = self.dev.alloc_tx_packet()?;

        Some(MiniTxToken {
            dev: self.dev,
            handle: Some(handle),
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1500;
        caps
    }
}
