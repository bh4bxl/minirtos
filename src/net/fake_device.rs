use crate::net::packet::{PacketHandle, PacketPool};
use crate::sys::{
    sync::{event::Event, message_queue::MessageQueue},
    synchronization::{CriticalSectionLock, critical_section},
};

const RX_QUEUE_SIZE: usize = 8;
const TX_QUEUE_SIZE: usize = 8;

pub struct FakeNetDevice {
    pool: CriticalSectionLock<PacketPool>,
    rx_queue: MessageQueue<PacketHandle, RX_QUEUE_SIZE>,
    tx_queue: MessageQueue<PacketHandle, TX_QUEUE_SIZE>,
    rx_event: Event,
}

#[allow(dead_code)]
impl FakeNetDevice {
    pub const fn new() -> Self {
        Self {
            pool: CriticalSectionLock::new(PacketPool::new()),
            rx_queue: MessageQueue::new(),
            tx_queue: MessageQueue::new(),
            rx_event: Event::new(false),
        }
    }

    /// Inject an RX Ethernet frame into the fake device.
    pub fn inject_rx(&self, frame: &[u8]) -> bool {
        let handle = critical_section(|cs| {
            self.pool.lock(cs, |pool| {
                let handle = pool.alloc()?;

                let Some(pkt) = pool.get_mut(handle) else {
                    pool.free(handle);
                    return None;
                };

                if pkt.extend_from_slice(frame).is_err() {
                    pool.free(handle);
                    return None;
                }

                Some(handle)
            })
        });

        let Some(handle) = handle else {
            return false;
        };

        if !self.rx_queue.try_send(handle) {
            critical_section(|cs| {
                self.pool.lock(cs, |pool| {
                    pool.free(handle);
                });
            });
            return false;
        }

        self.rx_event.signal();
        true
    }

    /// Wait until at least one RX packet is available.
    pub fn wait_rx(&self) {
        self.rx_event.wait();
    }

    /// Receive one packet handle. Blocking.
    pub fn recv(&self) -> PacketHandle {
        self.rx_queue.recv()
    }

    /// Receive one packet handle. Non-blocking.
    pub fn try_recv(&self) -> Option<PacketHandle> {
        self.rx_queue.try_recv()
    }

    /// Queue a TX packet handle. Blocking.
    pub fn send(&self, handle: PacketHandle) {
        self.tx_queue.send(handle);
    }

    /// Queue a TX packet handle. Non-blocking.
    pub fn try_send(&self, handle: PacketHandle) -> bool {
        self.tx_queue.try_send(handle)
    }

    /// Fetch one transmitted packet from the fake device side.
    pub fn take_tx(&self) -> Option<PacketHandle> {
        self.tx_queue.try_recv()
    }

    pub fn with_packet<R>(&self, handle: PacketHandle, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        critical_section(|cs| {
            self.pool.lock(cs, |pool| {
                let pkt = pool.get(handle)?;
                Some(f(pkt.as_slice()))
            })
        })
    }

    pub fn with_packet_mut<R>(
        &self,
        handle: PacketHandle,
        f: impl FnOnce(&mut [u8]) -> R,
    ) -> Option<R> {
        critical_section(|cs| {
            self.pool.lock(cs, |pool| {
                let pkt = pool.get_mut(handle)?;
                Some(f(pkt.as_mut_slice()))
            })
        })
    }

    pub fn free_packet(&self, handle: PacketHandle) {
        critical_section(|cs| {
            self.pool.lock(cs, |pool| {
                pool.free(handle);
            });
        });
    }

    pub fn free_count(&self) -> usize {
        critical_section(|cs| self.pool.lock(cs, |pool| pool.free_count()))
    }

    pub fn alloc_tx_packet(&self) -> Option<PacketHandle> {
        critical_section(|cs| self.pool.lock(cs, |pool| pool.alloc()))
    }

    pub fn with_packet_storage_mut<R>(
        &self,
        handle: PacketHandle,
        len: usize,
        f: impl FnOnce(&mut [u8]) -> R,
    ) -> Option<R> {
        critical_section(|cs| {
            self.pool.lock(cs, |pool| {
                let pkt = pool.get_mut(handle)?;

                if pkt.set_len(len).is_err() {
                    pool.free(handle);
                    return None;
                }

                Some(f(pkt.as_mut_slice()))
            })
        })
    }
}
