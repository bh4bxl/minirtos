use heapless::{Deque, Vec};

use crate::sys::{
    scheduler,
    synchronization::{CriticalSection, CriticalSectionLock, critical_section},
    syscall,
    task::TaskId,
};

const MAX_WAITERS: usize = 16;

struct MessageQueueInner<T, const N: usize> {
    buf: Deque<T, N>,
    recv_waiters: Vec<TaskId, MAX_WAITERS>,
    send_waiters: Vec<TaskId, MAX_WAITERS>,
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
                recv_waiters: Vec::new(),
                send_waiters: Vec::new(),
            }),
        }
    }

    /// Thread context only. Blocks if queue is full.
    pub fn send(&self, msg: T)
    where
        T: Copy,
    {
        let msg = msg;
        loop {
            let sent = critical_section(|cs| self.send_cs(cs, msg));

            if sent {
                return;
            }

            syscall::yield_now();
        }
    }

    /// ISR-safe / non-blocking.
    pub fn try_send(&self, msg: T) -> bool {
        critical_section(|cs| self.send_cs(cs, msg))
    }

    fn send_cs(&self, cs: &CriticalSection, msg: T) -> bool {
        self.inner.lock(cs, |inner| {
            if inner.buf.push_back(msg).is_ok() {
                if let Some(task_id) = inner.recv_waiters.pop() {
                    scheduler::scheduler().wake_task(cs, task_id);
                }
                true
            } else {
                let task_id = scheduler::scheduler().current_task_id(cs);

                if !inner.send_waiters.iter().any(|&id| id == task_id) {
                    let _ = inner.send_waiters.push(task_id);
                }

                scheduler::scheduler().block_current_task(cs);
                false
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

            syscall::yield_now();
        }
    }

    /// ISR-safe / non-blocking.
    pub fn try_recv(&self) -> Option<T>
    where
        T: Copy,
    {
        critical_section(|cs| self.recv_cs(cs))
    }

    fn recv_cs(&self, cs: &CriticalSection) -> Option<T> {
        self.inner.lock(cs, |inner| {
            if let Some(msg) = inner.buf.pop_front() {
                if let Some(task_id) = inner.send_waiters.pop() {
                    scheduler::scheduler().wake_task(cs, task_id);
                }
                Some(msg)
            } else {
                let task_id = scheduler::scheduler().current_task_id(cs);

                if !inner.recv_waiters.iter().any(|&id| id == task_id) {
                    let _ = inner.recv_waiters.push(task_id);
                }

                scheduler::scheduler().block_current_task(cs);
                None
            }
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
