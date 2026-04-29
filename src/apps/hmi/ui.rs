use core::cell::UnsafeCell;

use embedded_graphics::{
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

use crate::{
    gui::{self, display::FramebufferDisplay},
    sys::input::{InputEvent, Key},
};

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

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Main,
    Info,
}

#[derive(Clone, Copy)]
pub enum ButtonState {
    Normal,
    Focused,
    Pressed,
}

pub struct UiState {
    page: Page,
    focused: usize,
    pressed: bool,
}

impl UiState {
    pub const fn new() -> Self {
        Self {
            page: Page::Main,
            focused: 0,
            pressed: false,
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) {
        const COLS: usize = 2;
        const BUTTONS: usize = 4;

        match event {
            InputEvent::KeyDown(Key::Left) if self.page == Page::Main => {
                if self.focused % COLS > 0 {
                    self.focused -= 1;
                }
            }
            InputEvent::KeyDown(Key::Right) if self.page == Page::Main => {
                if self.focused % COLS + 1 < COLS && self.focused + 1 < BUTTONS {
                    self.focused += 1;
                }
            }
            InputEvent::KeyDown(Key::Up) if self.page == Page::Main => {
                if self.focused >= COLS {
                    self.focused -= COLS;
                }
            }
            InputEvent::KeyDown(Key::Down) if self.page == Page::Main => {
                if self.focused + COLS < BUTTONS {
                    self.focused += COLS;
                }
            }
            InputEvent::KeyDown(Key::Enter) | InputEvent::KeyDown(Key::A)
                if self.page == Page::Main =>
            {
                self.pressed = true;
            }
            InputEvent::KeyUp(Key::Enter) | InputEvent::KeyUp(Key::A)
                if self.page == Page::Main =>
            {
                self.pressed = false;

                if self.focused == 3 {
                    self.page = Page::Info;
                }
            }
            InputEvent::KeyDown(Key::B) => {
                self.page = Page::Main;
                self.pressed = false;
            }
            _ => {}
        }
    }

    pub fn button_state(&self, index: usize) -> ButtonState {
        if self.page != Page::Main {
            return ButtonState::Normal;
        }

        if index == self.focused {
            if self.pressed {
                ButtonState::Pressed
            } else {
                ButtonState::Focused
            }
        } else {
            ButtonState::Normal
        }
    }
}

fn button_style(state: ButtonState) -> PrimitiveStyle<Rgb565> {
    match state {
        ButtonState::Normal => PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::WHITE)
            .stroke_width(2)
            .fill_color(Rgb565::new(28, 28, 28))
            .build(),

        ButtonState::Focused => PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::YELLOW)
            .stroke_width(3)
            .fill_color(Rgb565::new(40, 40, 40))
            .build(),

        ButtonState::Pressed => PrimitiveStyleBuilder::new()
            .stroke_color(Rgb565::GREEN)
            .stroke_width(2)
            .fill_color(Rgb565::new(10, 60, 10))
            .build(),
    }
}

fn draw_sidebar<D: DrawTarget<Color = Rgb565>>(display: &mut D, text_style: MonoTextStyle<Rgb565>) {
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(5, 5), Size::new(40, 125)),
        Size::new(5, 5),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
    .draw(display)
    .ok();

    let text = "miniRTOS";
    for (i, c) in text.chars().enumerate() {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        Text::new(s, Point::new(20, 18 + i as i32 * 15), text_style)
            .draw(display)
            .ok();
    }
}

fn draw_button<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    rect: Rectangle,
    label: &str,
    state: ButtonState,
    text_style: MonoTextStyle<Rgb565>,
) {
    RoundedRectangle::with_equal_corners(rect, Size::new(15, 15))
        .into_styled(button_style(state))
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

fn draw_main_page<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    state: &UiState,
    text_style: MonoTextStyle<Rgb565>,
) {
    draw_button(
        display,
        Rectangle::new(Point::new(60, 15), Size::new(80, 40)),
        "Time",
        state.button_state(0),
        text_style,
    );

    draw_button(
        display,
        Rectangle::new(Point::new(150, 15), Size::new(80, 40)),
        "Task",
        state.button_state(1),
        text_style,
    );

    draw_button(
        display,
        Rectangle::new(Point::new(60, 70), Size::new(80, 40)),
        "Devs",
        state.button_state(2),
        text_style,
    );

    draw_button(
        display,
        Rectangle::new(Point::new(150, 70), Size::new(80, 40)),
        "Info",
        state.button_state(3),
        text_style,
    );
}

fn draw_info_page<D: DrawTarget<Color = Rgb565>>(display: &mut D) {
    Text::new(
        "miniRTOS",
        Point::new(60, 35),
        MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK),
    )
    .draw(display)
    .ok();

    Text::new(
        env!("CARGO_PKG_VERSION"),
        Point::new(60, 60),
        MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK),
    )
    .draw(display)
    .ok();

    Text::new(
        crate::sys::board::board().board_name(),
        Point::new(60, 85),
        MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK),
    )
    .draw(display)
    .ok();
}

pub fn main_windows(state: &UiState) {
    let lcd = gui::lcd_flush();

    let mut display =
        FramebufferDisplay::<LCD_WIDTH, LCD_HEIGHT, LCD_PIXELS>::new(lcd, UI_FRAME_BUF.get());

    RoundedRectangle::with_equal_corners(
        Rectangle::new(
            Point::new(0, 0),
            Size::new(LCD_WIDTH as u32, LCD_HEIGHT as u32),
        ),
        Size::new(0, 0),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
    .draw(&mut display)
    .ok();

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    draw_sidebar(&mut display, text_style);

    match state.page {
        Page::Main => draw_main_page(&mut display, state, text_style),
        Page::Info => draw_info_page(&mut display),
    }

    display.flush();
}
