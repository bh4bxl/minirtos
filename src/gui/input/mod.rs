pub mod input_manager;

use crate::sys::{
    device_driver::DevError,
    synchronization::{NullLock, interface::Mutex},
};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TouchEvent {
    Down { x: u16, y: u16 },
    Move { x: u16, y: u16 },
    Up,
}

pub const MAX_TOUCH_POINTS: usize = 10;

#[derive(Copy, Clone, Debug)]
pub struct TouchPoint {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub size: u16,
}

impl TouchPoint {
    pub const EMPTY: Self = Self {
        id: 0,
        x: 0,
        y: 0,
        size: 0,
    };
}

#[derive(Copy, Clone, Debug)]
pub struct TouchReport {
    pub count: usize,
    pub points: [TouchPoint; MAX_TOUCH_POINTS],
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Idle = 0,
    Pressed = 1,
    Hold = 2,
    Released = 3,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyValueReport {
    pub state: KeyState,
    pub key: u8,
}

impl KeyValueReport {
    pub fn from_bytes(data: [u8; 2]) -> Self {
        Self {
            state: match data[0] {
                1 => KeyState::Pressed,
                2 => KeyState::Hold,
                3 => KeyState::Released,
                _ => KeyState::Idle,
            },
            key: data[1],
        }
    }
}

pub mod interface {
    use crate::sys::device_driver::DevError;

    pub trait TouchDevice {
        fn read_point(&self) -> Result<Option<super::TouchReport>, DevError>;
    }

    pub trait KeyboardDevice {
        fn read_key_value(&self) -> Result<Option<super::KeyValueReport>, DevError>;
    }
}

struct NullTouch;

impl interface::TouchDevice for NullTouch {
    fn read_point(&self) -> Result<Option<TouchReport>, DevError> {
        Err(DevError::NoSuchDevice)
    }
}

const NULL_TOUCH: NullTouch = NullTouch;

static CURR_TOUCH: NullLock<&'static (dyn interface::TouchDevice + Sync)> =
    NullLock::new(&NULL_TOUCH);

#[allow(dead_code)]
/// Register a new touch.
pub fn register_touch(new_touch: &'static (dyn interface::TouchDevice + Sync)) {
    CURR_TOUCH.lock(|con| *con = new_touch);
}

/// Return a reference to the currently registered touch device.
pub fn touch() -> &'static dyn interface::TouchDevice {
    CURR_TOUCH.lock(|con| *con)
}

struct NullKeyboard;

impl interface::KeyboardDevice for NullKeyboard {
    fn read_key_value(&self) -> Result<Option<self::KeyValueReport>, DevError> {
        Err(DevError::NoSuchDevice)
    }
}

const NULL_KEYBOARD: NullKeyboard = NullKeyboard;

static CURR_KEYBOARD: NullLock<&'static (dyn interface::KeyboardDevice + Sync)> =
    NullLock::new(&NULL_KEYBOARD);

#[allow(dead_code)]
/// Register a new keyboard
pub fn register_keyboard(new_kbd: &'static (dyn interface::KeyboardDevice + Sync)) {
    CURR_KEYBOARD.lock(|con| *con = new_kbd);
}

/// Return a reference to the currently registered keyboard device.
pub fn keyboard() -> &'static dyn interface::KeyboardDevice {
    CURR_KEYBOARD.lock(|con| *con)
}
