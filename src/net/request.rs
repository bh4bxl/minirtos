use core::net::{Ipv4Addr, SocketAddrV4};

use crate::sys::{
    sync::Event,
    synchronization::{CriticalSectionLock, critical_section},
};

use super::{
    BufferId, NetError, NetResult, SocketId, service::FixedStr,
    service::network_task::NET_CMD_QUEUE,
};

const MAX_REQUESTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RequestId {
    index: u8,
    generation: u16,
}

impl RequestId {
    pub const fn new(index: usize, generation: u16) -> Self {
        Self {
            index: index as u8,
            generation,
        }
    }

    pub const fn index(self) -> usize {
        self.index as usize
    }

    pub const fn generation(self) -> u16 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NetCommand {
    DnsResolve {
        request: RequestId,
        hostname: FixedStr<128>,
        timeout_ms: u64,
    },
    IcmpEcho {
        request: RequestId,
        target: Ipv4Addr,
        timeout_ms: u64,
    },
    TcpOpen {
        request: RequestId,
    },
    TcpConnect {
        request: RequestId,
        socket: SocketId,
        remote: SocketAddrV4,
        timeout_ms: u64,
    },
    TcpSend {
        request: RequestId,
        socket: SocketId,
        buffer: BufferId,
        len: usize,
        timeout_ms: u64,
    },
    TcpRecv {
        request: RequestId,
        socket: SocketId,
        buffer: BufferId,
        max_len: usize,
        timeout_ms: u64,
    },
    TcpClose {
        request: RequestId,
        socket: SocketId,
    },
    TcpAbort {
        socket: SocketId,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum NetResponse {
    DnsResolved {
        addr: Ipv4Addr,
    },
    IcmpReply {
        addr: Ipv4Addr,
        sequence: u16,
        bytes: usize,
        rtt_ms: u64,
    },
    TcpOpened {
        socket: SocketId,
    },
    TcpConnected,
    TcpSent {
        len: usize,
    },
    TcpReceived {
        len: usize,
    },
    TcpClosed,
    Error(NetError),
}

#[derive(Clone, Copy)]
struct RequestSlotInner {
    generation: u16,
    in_use: bool,
    response: Option<NetResponse>,
}

impl RequestSlotInner {
    const fn new() -> Self {
        Self {
            generation: 0,
            in_use: false,
            response: None,
        }
    }
}

struct RequestSlot {
    inner: CriticalSectionLock<RequestSlotInner>,
    completed: Event,
}

impl RequestSlot {
    const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(RequestSlotInner::new()),
            completed: Event::new(false),
        }
    }

    fn allocate(&self, index: usize) -> Option<RequestId> {
        let request = critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if inner.in_use {
                    return None;
                }

                inner.generation = inner.generation.wrapping_add(1);

                /*
                 * Generation zero may be reserved as an invalid/default
                 * value if desired.
                 */
                if inner.generation == 0 {
                    inner.generation = 1;
                }

                inner.in_use = true;
                inner.response = None;

                Some(RequestId::new(index, inner.generation))
            })
        });

        if request.is_some() {
            /*
             * A correctly completed request consumes its signal in wait().
             * Drain any stale signal defensively before reusing this slot.
             *
             * The slot is now allocated, so no other request can complete
             * it while this cleanup is performed.
             */
            if self.completed.is_signaled() {
                self.completed.wait();
            }
        }

        request
    }

    fn complete(&self, request: RequestId, response: NetResponse) -> bool {
        let completed = critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if !inner.in_use || inner.generation != request.generation {
                    return false;
                }

                /*
                 * A request must complete exactly once.
                 */
                if inner.response.is_some() {
                    return false;
                }

                inner.response = Some(response);
                true
            })
        });

        /*
         * Never signal while holding RequestSlotInner's lock.
         */
        if completed {
            self.completed.signal();
        }

        completed
    }

    fn wait(&self, request: RequestId) -> NetResult<NetResponse> {
        /*
         * Event handles both orderings safely:
         *
         * - network task completes first, signal is remembered;
         * - caller waits first, task is blocked and later awakened.
         */
        self.completed.wait();

        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if !inner.in_use || inner.generation != request.generation {
                    return Err(NetError::InvalidRequest);
                }

                inner.response.take().ok_or(NetError::Internal)
            })
        })
    }

    fn release(&self, request: RequestId) {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if inner.in_use && inner.generation == request.generation {
                    inner.in_use = false;
                    inner.response = None;
                }
            })
        });
    }
}

struct RequestTable {
    slots: [RequestSlot; MAX_REQUESTS],
}

impl RequestTable {
    const fn new() -> Self {
        Self {
            slots: [const { RequestSlot::new() }; MAX_REQUESTS],
        }
    }

    fn allocate(&self) -> NetResult<RequestGuard> {
        for (index, slot) in self.slots.iter().enumerate() {
            if let Some(id) = slot.allocate(index) {
                return Ok(RequestGuard { id });
            }
        }

        Err(NetError::NoRequestAvailable)
    }

    fn slot(&self, request: RequestId) -> NetResult<&RequestSlot> {
        self.slots
            .get(request.index())
            .ok_or(NetError::InvalidRequest)
    }

    fn complete(&self, request: RequestId, response: NetResponse) -> NetResult<()> {
        let slot = self.slot(request)?;

        if slot.complete(request, response) {
            Ok(())
        } else {
            Err(NetError::InvalidRequest)
        }
    }
}

static REQUEST_TABLE: RequestTable = RequestTable::new();

struct RequestGuard {
    id: RequestId,
}

impl RequestGuard {
    const fn id(&self) -> RequestId {
        self.id
    }

    fn wait(&self) -> NetResult<NetResponse> {
        REQUEST_TABLE.slot(self.id)?.wait(self.id)
    }
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        if let Ok(slot) = REQUEST_TABLE.slot(self.id) {
            slot.release(self.id);
        }
    }
}

pub(super) struct RequestManager;

impl RequestManager {
    pub fn submit<F>(build: F) -> NetResult<NetResponse>
    where
        F: FnOnce(RequestId) -> NetCommand,
    {
        let request = REQUEST_TABLE.allocate()?;

        /*
         * MessageQueue::send() blocks when the queue is full. This is fine
         * for the initial blocking network API.
         */
        NET_CMD_QUEUE.send(build(request.id()));

        request.wait()
    }

    pub(crate) fn send(command: NetCommand) {
        /*
         * Used for commands that intentionally have no response, such as
         * TcpAbort from Drop.
         */
        NET_CMD_QUEUE.send(command);
    }

    pub(crate) fn complete(request: RequestId, response: NetResponse) -> NetResult<()> {
        REQUEST_TABLE.complete(request, response)
    }
}

/*
 * Convenience function for the network task.
 */
pub(super) fn complete_request(request: RequestId, response: NetResponse) {
    if RequestManager::complete(request, response).is_err() {
        defmt::warn!("NET: invalid or stale request completion");
    }
}
