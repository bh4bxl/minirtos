use crate::sys::synchronization::{NullLock, interface::Mutex};

pub mod display;

#[allow(dead_code)]
pub mod interface {
    pub trait LcdFlush {
        fn set_window(&self, x: u16, y: u16, w: u16, h: u16);

        fn flush_buf(&self, data: &[u8]);

        fn flush_buf_u16(&self, data: &[u16]);
    }
}

/// A placeholder.
struct NullLcdFlush;

impl interface::LcdFlush for NullLcdFlush {
    fn set_window(&self, _x: u16, _y: u16, _w: u16, _h: u16) {}

    fn flush_buf(&self, _data: &[u8]) {}

    fn flush_buf_u16(&self, _data: &[u16]) {}
}

const NULL_LCD_FLUSH: NullLcdFlush = NullLcdFlush;

/// A reference to the global flush.
static CURR_FLUSH: NullLock<&'static (dyn interface::LcdFlush + Sync)> =
    NullLock::new(&NULL_LCD_FLUSH);

/// Register a new flush.
pub fn register_lcd_flush(new_console: &'static (dyn interface::LcdFlush + Sync)) {
    CURR_FLUSH.lock(|con| *con = new_console);
}

/// Return a reference to the currently registered flush.
pub fn lcd_flush() -> &'static dyn interface::LcdFlush {
    CURR_FLUSH.lock(|con| *con)
}
