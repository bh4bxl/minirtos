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

pub mod interface {
    use crate::sys::device_driver::DevError;

    pub trait TouchDevice {
        fn read_point(&self) -> Result<Option<super::TouchReport>, DevError>;
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

/// Register a new flush.
pub fn register_touch(new_touch: &'static (dyn interface::TouchDevice + Sync)) {
    CURR_TOUCH.lock(|con| *con = new_touch);
}

/// Return a reference to the currently registered flush.
pub fn touch() -> &'static dyn interface::TouchDevice {
    CURR_TOUCH.lock(|con| *con)
}
