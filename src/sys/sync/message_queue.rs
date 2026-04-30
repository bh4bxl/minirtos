use heapless::Deque;

use crate::sys::{
    sync::wait_queue::WaitQueue,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
};

enum SendResult<T> {
    Sent,
    Full(T),
}

struct MessageQueueInner<T, const N: usize> {
    buf: Deque<T, N>,
    recv_waiters: WaitQueue,
    send_waiters: WaitQueue,
}

pub struct MessageQueue<T, const N: usize> {
    inner: CriticalSectionLock<MessageQueueInner<T, N>>,
}

#[allow(dead_code)]
impl<T, const N: usize> MessageQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(MessageQueueInner {
                buf: Deque::new(),
                recv_waiters: WaitQueue::new(),
                send_waiters: WaitQueue::new(),
            }),
        }
    }

    /// Thread context only. Blocks if queue is full.
    pub fn send(&self, mut msg: T)
    where
        T: Copy,
    {
        loop {
            match critical_section(|cs| self.send_cs(cs, msg)) {
                SendResult::Sent => return,
                SendResult::Full(returned_msg) => {
                    msg = returned_msg;
                }
            }
        }
    }

    /// ISR-safe / non-blocking.
    pub fn try_send(&self, msg: T) -> bool {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| match inner.buf.push_back(msg) {
                Ok(()) => {
                    inner.recv_waiters.wake_one(cs);
                    true
                }
                Err(_) => false,
            })
        })
    }

    fn send_cs(&self, cs: &CriticalSection, msg: T) -> SendResult<T> {
        self.inner.lock(cs, |inner| match inner.buf.push_back(msg) {
            Ok(()) => {
                inner.recv_waiters.wake_one(cs);
                SendResult::Sent
            }
            Err(msg) => {
                inner.send_waiters.block_current(cs);
                SendResult::Full(msg)
            }
        })
    }

    /// Thread context only. Blocks if queue is empty.
    pub fn recv(&self) -> T
    where
        T: Copy,
    {
        loop {
            if let Some(msg) = critical_section(|cs| self.recv_cs(cs)) {
                return msg;
            }
        }
    }

    /// ISR-safe / non-blocking.
    pub fn try_recv(&self) -> Option<T>
    where
        T: Copy,
    {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                let msg = inner.buf.pop_front();

                if msg.is_some() {
                    inner.send_waiters.wake_one(cs);
                }

                msg
            })
        })
    }

    fn recv_cs(&self, cs: &CriticalSection) -> Option<T> {
        self.inner.lock(cs, |inner| {
            if let Some(msg) = inner.buf.pop_front() {
                inner.send_waiters.wake_one(cs);
                Some(msg)
            } else {
                inner.recv_waiters.block_current(cs);
                None
            }
        })
    }

    fn try_recv_cs(&self, cs: &CriticalSection) -> Option<T> {
        self.inner.lock(cs, |inner| {
            let msg = inner.buf.pop_front();

            if msg.is_some() {
                inner.send_waiters.wake_one(cs);
            }

            msg
        })
    }

    pub fn len(&self) -> usize {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.buf.len()))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_full(&self) -> bool {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.buf.is_full()))
    }
}
