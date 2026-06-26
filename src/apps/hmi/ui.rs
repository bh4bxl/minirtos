use core::cell::UnsafeCell;

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, RgbColor, Size},
    primitives::Rectangle,
};

use crate::gui::{
    self, desktop::Desktop, display::FramebufferDisplay, draw::DrawContext, theme::Theme,
    widget::Widget,
};

#[cfg(feature = "pico2w-ws-lcd114")]
const LCD_SZIE: (usize, usize) = (240, 135);

#[cfg(feature = "pico2w-52pi")]
const LCD_SZIE: (usize, usize) = (480, 320);

#[cfg(feature = "pico2w-picocalc")]
const LCD_SZIE: (usize, usize) = (320, 320);

const LCD_WIDTH: usize = LCD_SZIE.0;
const LCD_HEIGHT: usize = LCD_SZIE.1;
const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

struct UiFrameBuf(UnsafeCell<[u16; LCD_PIXELS]>);

unsafe impl Sync for UiFrameBuf {}

impl UiFrameBuf {
    fn get(&self) -> &'static mut [u16; LCD_PIXELS] {
        unsafe { &mut *self.0.get() }
    }
}

static UI_FRAME_BUF: UiFrameBuf = UiFrameBuf(UnsafeCell::new([0; LCD_PIXELS]));

pub fn desktop() {
    let lcd = gui::lcd_flush();

    let mut display =
        FramebufferDisplay::<LCD_WIDTH, LCD_HEIGHT, LCD_PIXELS>::new(lcd, UI_FRAME_BUF.get());

    display.clear(Rgb565::BLACK).ok();

    let theme = Theme::classic(
        Rgb565::BLACK,
        Rgb565::WHITE,
        Rgb565::new(128, 128, 128),
        Rgb565::BLACK,
    );

    let mut ctx = DrawContext::new(&mut display, &theme);

    let desktop = Desktop::<32>::new(Rectangle::new(
        Point::new(0, 0),
        Size::new(LCD_WIDTH as u32, LCD_HEIGHT as u32),
    ))
    .with_title("miniRTOS");
    desktop.draw(&mut ctx).ok();

    display.flush();
}
