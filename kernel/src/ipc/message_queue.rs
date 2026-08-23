use heapless::Deque;

pub(super) struct MessageQueue<T, const N: usize> {
    buf: Deque<T, N>,
}

impl<T, const N: usize> MessageQueue<T, N> {
    pub const fn new() -> Self {
        Self { buf: Deque::new() }
    }

    pub fn push(&mut self, msg: T) -> Result<(), T> {
        self.buf.push_back(msg)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.buf.pop_front()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buf.is_full()
    }
}
