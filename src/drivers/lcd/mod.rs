pub mod st7789vw;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum DisplayRoation {
    Roation0,
    Roation90,
    Roation180,
    Roation270,
}

#[derive(Clone, Copy, Debug)]
pub struct LcdConfig {
    pub ration: DisplayRoation,
    pub width: u32,
    pub height: u32,
    pub x_offset: u32,
    pub y_offset: u32,
}

impl Default for LcdConfig {
    fn default() -> Self {
        Self {
            ration: DisplayRoation::Roation90,
            width: 240,
            height: 135,
            x_offset: 40,
            y_offset: 53,
        }
    }
}

#[allow(dead_code)]
pub mod interface {
    use crate::sys::device_driver::DevError;

    pub trait Lcd {
        fn config(&self, config: &super::LcdConfig) -> Result<(), DevError>;

        fn display_on(&self) -> Result<(), DevError>;

        fn set_window(&self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DevError>;

        fn flush_buf(&self, x: u16, y: u16, w: u16, h: u16, buf: &[u8]) -> Result<(), DevError>;
    }
}
