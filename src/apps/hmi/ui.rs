use core::cell::UnsafeCell;

use embedded_graphics::{
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

use crate::gui::{self, display::FramebufferDisplay};

const LCD_WIDTH: usize = 240;
const LCD_HEIGHT: usize = 135;
const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

struct UiFrameBuf(UnsafeCell<[u16; LCD_PIXELS]>);

unsafe impl Sync for UiFrameBuf {}

impl UiFrameBuf {
    fn get(&self) -> &'static mut [u16; LCD_PIXELS] {
        unsafe { &mut *self.0.get() }
    }
}

static UI_FRAME_BUF: UiFrameBuf = UiFrameBuf(UnsafeCell::new([0; LCD_PIXELS]));

#[derive(Clone, Copy)]
pub enum ButtonState {
    Normal,
    Focused,
    Pressed,
}

fn button_style(state: ButtonState) -> PrimitiveStyle<Rgb565> {
    match state {
        ButtonState::Normal => PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::WHITE)
            .stroke_width(2)
            .fill_color(Rgb565::new(28, 28, 28)) // 深灰
            .build(),

        ButtonState::Focused => PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::YELLOW)
            .stroke_width(3)
            .fill_color(Rgb565::new(40, 40, 40)) // 稍亮
            .build(),

        ButtonState::Pressed => PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::GREEN)
            .stroke_width(2)
            .fill_color(Rgb565::new(10, 60, 10)) // 深绿
            .build(),
    }
}

fn draw_button<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    rect: Rectangle,
    label: &str,
    state: ButtonState,
    text_style: MonoTextStyle<Rgb565>,
) {
    let style = button_style(state);

    RoundedRectangle::with_equal_corners(rect, Size::new(15, 15))
        .into_styled(style)
        .draw(display)
        .ok();

    let center = rect.center();

    Text::with_alignment(
        label,
        Point::new(center.x, center.y + 6),
        text_style,
        Alignment::Center,
    )
    .draw(display)
    .ok();
}

/// The UI main
pub fn main_windows() {
    let lcd = gui::lcd_flush();

    let mut display =
        FramebufferDisplay::<LCD_WIDTH, LCD_HEIGHT, LCD_PIXELS>::new(lcd, UI_FRAME_BUF.get());

    let bg_style = PrimitiveStyle::with_fill(Rgb565::WHITE);

    RoundedRectangle::with_equal_corners(
        Rectangle::new(
            Point::new(0, 0),
            Size::new(LCD_WIDTH as u32, LCD_HEIGHT as u32),
        ),
        Size::new(0, 0),
    )
    .into_styled(bg_style)
    .draw(&mut display)
    .ok();

    let sidebar_style = PrimitiveStyle::with_fill(Rgb565::BLUE);
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(5, 5), Size::new(40, 125)),
        Size::new(5, 5),
    )
    .into_styled(sidebar_style)
    .draw(&mut display)
    .ok();

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    let text = "miniRTOS";
    for (i, c) in text.chars().enumerate() {
        let mut buf = [0u8; 4]; // UTF-8 max 4 bytes
        let s = c.encode_utf8(&mut buf);

        Text::new(s, Point::new(20, 18 + i as i32 * 15), text_style)
            .draw(&mut display)
            .ok();
    }

    draw_button(
        &mut display,
        Rectangle::new(Point::new(60, 15), Size::new(80, 40)),
        "Time",
        ButtonState::Focused,
        text_style,
    );

    draw_button(
        &mut display,
        Rectangle::new(Point::new(150, 15), Size::new(80, 40)),
        "Task",
        ButtonState::Normal,
        text_style,
    );

    draw_button(
        &mut display,
        Rectangle::new(Point::new(60, 70), Size::new(80, 40)),
        "Devs",
        ButtonState::Pressed,
        text_style,
    );

    draw_button(
        &mut display,
        Rectangle::new(Point::new(150, 70), Size::new(80, 40)),
        "Info",
        ButtonState::Normal,
        text_style,
    );

    display.flush();
}
