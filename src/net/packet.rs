pub const PACKET_BUF_SIZE: usize = 1536; // Ethernet MTU + headers
pub const PACKET_POOL_SIZE: usize = 8;

/// A fixed-size packet buffer
#[derive(Clone, Copy)]
pub struct PacketBuf {
    len: usize,
    data: [u8; PACKET_BUF_SIZE],
}

#[allow(dead_code)]
impl PacketBuf {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [0; PACKET_BUF_SIZE],
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        PACKET_BUF_SIZE
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn extend_from_slice(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if bytes.len() > (PACKET_BUF_SIZE - self.len) {
            return Err(());
        }

        let start = self.len;
        let end = start + bytes.len();

        self.data[start..end].copy_from_slice(bytes);
        self.len = end;

        Ok(())
    }

    pub fn set_len(&mut self, len: usize) -> Result<(), ()> {
        if len > PACKET_BUF_SIZE {
            return Err(());
        }

        self.len = len;
        Ok(())
    }
}

/// Handle to a packet in the pool
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PacketHandle(pub usize);

/// Fixed-size packet pool (no malloc)
pub struct PacketPool {
    bufs: [PacketBuf; PACKET_POOL_SIZE],
    used: [bool; PACKET_POOL_SIZE],
}

#[allow(dead_code)]
impl PacketPool {
    pub const fn new() -> Self {
        Self {
            bufs: [PacketBuf::new(); PACKET_POOL_SIZE],
            used: [false; PACKET_POOL_SIZE],
        }
    }

    /// Allocate a packet buffer
    pub fn alloc(&mut self) -> Option<PacketHandle> {
        for i in 0..PACKET_POOL_SIZE {
            if !self.used[i] {
                self.used[i] = true;
                self.bufs[i].clear();
                return Some(PacketHandle(i));
            }
        }

        None
    }

    /// Free a packet buffer
    pub fn free(&mut self, handle: PacketHandle) {
        let idx = handle.0;

        if idx < PACKET_POOL_SIZE {
            self.bufs[idx].clear();
            self.used[idx] = false;
        }
    }

    pub fn get(&self, handle: PacketHandle) -> Option<&PacketBuf> {
        self.bufs.get(handle.0)
    }

    pub fn get_mut(&mut self, handle: PacketHandle) -> Option<&mut PacketBuf> {
        self.bufs.get_mut(handle.0)
    }

    pub fn free_count(&self) -> usize {
        let mut count = 0;
        let mut i = 0;

        while i < PACKET_POOL_SIZE {
            if !self.used[i] {
                count += 1;
            }
            i += 1;
        }

        count
    }
}
