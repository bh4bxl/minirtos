use crate::sys::synchronization::{CriticalSectionLock, critical_section};

use super::{NetError, NetResult};

pub(super) const NET_BUFFER_SIZE: usize = 512;
const NET_BUFFER_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferId(u8);

impl BufferId {
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

struct BufferPoolInner {
    used: [bool; NET_BUFFER_COUNT],
    len: [usize; NET_BUFFER_COUNT],
    data: [[u8; NET_BUFFER_SIZE]; NET_BUFFER_COUNT],
}

impl BufferPoolInner {
    const fn new() -> Self {
        Self {
            used: [false; NET_BUFFER_COUNT],
            len: [0; NET_BUFFER_COUNT],
            data: [[0; NET_BUFFER_SIZE]; NET_BUFFER_COUNT],
        }
    }

    fn allocate(&mut self) -> Option<BufferId> {
        for (index, used) in self.used.iter_mut().enumerate() {
            if !*used {
                *used = true;
                self.len[index] = 0;
                return Some(BufferId::new(index as u8));
            }
        }
        None
    }

    fn validate(&self, id: BufferId) -> NetResult<usize> {
        let index = id.index();
        if index >= NET_BUFFER_COUNT || !self.used[index] {
            return Err(NetError::InvalidBuffer);
        }
        Ok(index)
    }

    fn release(&mut self, id: BufferId) {
        let index = id.index();
        if index < NET_BUFFER_COUNT {
            self.used[index] = false;
            self.len[index] = 0;
        }
    }
}

static BUFFER_POOL: CriticalSectionLock<BufferPoolInner> =
    CriticalSectionLock::new(BufferPoolInner::new());

pub struct NetBuffer {
    id: BufferId,
}

impl NetBuffer {
    pub fn allocate() -> NetResult<Self> {
        critical_section(|cs| {
            BUFFER_POOL.lock(cs, |pool| {
                pool.allocate()
                    .map(|id| Self { id })
                    .ok_or(NetError::NoBufferAvailable)
            })
        })
    }

    pub const fn id(&self) -> BufferId {
        self.id
    }

    pub const fn capacity(&self) -> usize {
        NET_BUFFER_SIZE
    }

    pub fn len(&self) -> NetResult<usize> {
        critical_section(|cs| {
            BUFFER_POOL.lock(cs, |pool| {
                let index = pool.validate(self.id)?;
                Ok(pool.len[index])
            })
        })
    }

    pub(crate) fn write(&mut self, data: &[u8]) -> NetResult<usize> {
        critical_section(|cs| {
            BUFFER_POOL.lock(cs, |pool| {
                let index = pool.validate(self.id)?;
                let count = data.len().min(NET_BUFFER_SIZE);
                pool.data[index][..count].copy_from_slice(&data[..count]);
                pool.len[index] = count;
                Ok(count)
            })
        })
    }

    pub(crate) fn read(&self, output: &mut [u8]) -> NetResult<usize> {
        critical_section(|cs| {
            BUFFER_POOL.lock(cs, |pool| {
                let index = pool.validate(self.id)?;
                let count = output.len().min(pool.len[index]);
                output[..count].copy_from_slice(&pool.data[index][..count]);
                Ok(count)
            })
        })
    }
}

impl Drop for NetBuffer {
    fn drop(&mut self) {
        critical_section(|cs| BUFFER_POOL.lock(cs, |pool| pool.release(self.id)))
    }
}

pub(crate) fn with_buffer<R>(id: BufferId, f: impl FnOnce(&[u8]) -> R) -> NetResult<R> {
    critical_section(|cs| {
        BUFFER_POOL.lock(cs, |pool| {
            let index = pool.validate(id)?;
            Ok(f(&pool.data[index][..pool.len[index]]))
        })
    })
}

pub(crate) fn with_buffer_mut<R>(
    id: BufferId,
    f: impl FnOnce(&mut [u8], &mut usize) -> R,
) -> NetResult<R> {
    critical_section(|cs| {
        BUFFER_POOL.lock(cs, |pool| {
            let index = pool.validate(id)?;

            let result = f(&mut pool.data[index], &mut pool.len[index]);

            pool.len[index] = pool.len[index].min(NET_BUFFER_SIZE);

            Ok(result)
        })
    })
}
