use crate::synchronization::{CriticalSection, CriticalSectionLock, WaitQueue, critical_section};

use super::{Message, MessageQueue};

const ENDPOINT_QUEUE_CAPACITY: usize = 8;

struct EndpointInner {
    queue: MessageQueue<Message, ENDPOINT_QUEUE_CAPACITY>,
    recv_waiters: WaitQueue,
    send_waiters: WaitQueue,
}

pub(crate) struct Endpoint {
    inner: CriticalSectionLock<EndpointInner>,
}

impl Endpoint {
    pub const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(EndpointInner {
                queue: MessageQueue::new(),
                recv_waiters: WaitQueue::new(),
                send_waiters: WaitQueue::new(),
            }),
        }
    }

    pub fn try_send(&self, msg: Message) -> Result<(), Message> {
        critical_section(|cs| self.try_send_cs(cs, msg))
    }

    pub(crate) fn try_send_cs(&self, cs: &CriticalSection, msg: Message) -> Result<(), Message> {
        self.inner.lock(cs, |inner| match inner.queue.push(msg) {
            Ok(()) => {
                inner.recv_waiters.wake_one(cs);
                Ok(())
            }

            Err(msg) => Err(msg),
        })
    }

    pub fn try_recv(&self) -> Option<Message> {
        critical_section(|cs| self.try_recv_cs(cs))
    }

    pub(crate) fn try_recv_cs(&self, cs: &CriticalSection) -> Option<Message> {
        self.inner.lock(cs, |inner| {
            let msg = inner.queue.pop();

            if msg.is_some() {
                inner.send_waiters.wake_one(cs);
            }

            msg
        })
    }

    // Blocking support

    pub(crate) fn block_receiver_cs(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            inner.recv_waiters.block_current(cs);
        });
    }

    pub(crate) fn block_sender_cs(&self, cs: &CriticalSection) {
        self.inner.lock(cs, |inner| {
            inner.send_waiters.block_current(cs);
        });
    }

    // State

    pub fn len(&self) -> usize {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.queue.len()))
    }

    pub fn is_empty(&self) -> bool {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.queue.is_empty()))
    }

    pub fn is_full(&self) -> bool {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.queue.is_full()))
    }

    pub fn receiver_waiter_count(&self) -> usize {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.recv_waiters.len()))
    }

    pub fn sender_waiter_count(&self) -> usize {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.send_waiters.len()))
    }
}
