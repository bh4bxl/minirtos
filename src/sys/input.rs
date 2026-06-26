#![allow(dead_code)]
use crate::sys::synchronization::{IrqSafeNullLock, interface::Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Symbols
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Semicolon,
    Apostrophe,
    Comma,
    Dot,
    Slash,
    Backslash,
    Grave,

    // Controls
    Enter,
    Esc,
    Backspace,
    Tab,
    Space,

    // Arrows
    Up,
    Down,
    Left,
    Right,

    // Navigation
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,

    // Modifiers
    ShiftL,
    ShiftR,
    Ctrl,
    Alt,
    Fn,

    // Function
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,

    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchPoint {
    pub id: u8,
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoystickState {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadButton {
    A,
    B,
    X,
    Y,

    Start,
    Select,

    L1,
    R1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GamepadAxis {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    KeyDown(Key),
    KeyUp(Key),
    KeyHold(Key),

    TouchDown(TouchPoint),
    TouchMove(TouchPoint),
    TouchUp(TouchPoint),

    JoystickMove(JoystickState),

    GamepadButtonDown(GamepadButton),
    GamepadButtonUp(GamepadButton),
    GamepadAxis(GamepadAxis),
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
