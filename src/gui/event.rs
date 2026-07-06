use embedded_graphics::geometry::Point;

use crate::{
    gui::event::EventResult::Consumed,
    sys::input::{InputEvent, JoystickState, Key, TouchPoint},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuiTouchEvent {
    pub id: u8,
    pub pos: Point,
}

impl From<TouchPoint> for GuiTouchEvent {
    fn from(p: TouchPoint) -> Self {
        Self {
            id: p.id,
            pos: Point {
                x: p.x as i32,
                y: p.y as i32,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuiEvent {
    KeyDown(Key),
    KeyUp(Key),
    KeyHold(Key),

    TouchDown(GuiTouchEvent),
    TouchMove(GuiTouchEvent),
    TouchUp(GuiTouchEvent),

    JoystickMove(JoystickState),

    Tick(u64),
}

impl TryFrom<InputEvent> for GuiEvent {
    type Error = ();

    fn try_from(event: InputEvent) -> Result<Self, Self::Error> {
        match event {
            InputEvent::KeyDown(k) => Ok(Self::KeyDown(k)),
            InputEvent::KeyUp(k) => Ok(Self::KeyUp(k)),
            InputEvent::KeyHold(k) => Ok(Self::KeyHold(k)),
            InputEvent::TouchDown(p) => Ok(Self::TouchDown(p.into())),
            InputEvent::TouchMove(p) => Ok(Self::TouchMove(p.into())),
            InputEvent::TouchUp(p) => Ok(Self::TouchUp(p.into())),
            InputEvent::JoystickMove(p) => Ok(Self::JoystickMove(p)),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventResult {
    Ignored,
    Consumed,
    NeedRedraw,
}

impl EventResult {
    pub fn is_handled(&self) -> bool {
        self == &Consumed
    }
}
