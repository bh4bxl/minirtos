#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SocketId {
    index: u8,
    generation: u16,
}

impl SocketId {
    pub(crate) const fn new(index: u8, generation: u16) -> Self {
        Self { index, generation }
    }

    pub(crate) const fn index(self) -> usize {
        self.index as usize
    }

    pub(crate) const fn generation(self) -> u16 {
        self.generation
    }
}
