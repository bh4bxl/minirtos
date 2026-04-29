use crate::sys::synchronization::{IrqSafeNullLock, interface::Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    A,
    B,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    KeyDown(Key),
    KeyUp(Key),
}

const INPUT_QUEUE_SIZE: usize = 16;

struct InputQueueInner {
    buf: [Option<InputEvent>; INPUT_QUEUE_SIZE],
    head: usize,
    tail: usize,
    len: usize,
}

impl InputQueueInner {
    const fn new() -> Self {
        Self {
            buf: [None; INPUT_QUEUE_SIZE],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    fn push(&mut self, event: InputEvent) -> Result<(), &'static str> {
        if self.len == INPUT_QUEUE_SIZE {
            return Err("Input queue is full");
        }

        self.buf[self.tail] = Some(event);
        self.tail = (self.tail + 1) % INPUT_QUEUE_SIZE;
        self.len += 1;

        Ok(())
    }

    fn pop(&mut self) -> Option<InputEvent> {
        if self.len == 0 {
            return None;
        }

        let event = self.buf[self.head];
        self.buf[self.head] = None;
        self.head = (self.head + 1) % INPUT_QUEUE_SIZE;
        self.len -= 1;
        event
    }
}

pub struct Input {
    inner: IrqSafeNullLock<InputQueueInner>,
}

#[allow(dead_code)]
impl Input {
    pub const fn new() -> Self {
        Self {
            inner: IrqSafeNullLock::new(InputQueueInner::new()),
        }
    }

    pub fn push_event(&self, event: InputEvent) -> Result<(), &'static str> {
        self.inner.lock(|inner| inner.push(event))
    }

    pub fn poll_event(&self) -> Option<InputEvent> {
        self.inner.lock(|inner| inner.pop())
    }

    pub fn has_event(&self) -> bool {
        self.inner.lock(|inner| inner.len > 0)
    }
}

static INPUT_QUEUE: Input = Input::new();

pub fn input_queue() -> &'static Input {
    &INPUT_QUEUE
}
