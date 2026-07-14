use embedded_graphics::pixelcolor::{Rgb565, RgbColor};

use crate::gui::theme::Palette;

pub struct ClassicRgb565Palette;

impl ClassicRgb565Palette {
    pub fn light() -> Palette<Rgb565> {
        Palette {
            background: Rgb565::BLACK,
            surface: Rgb565::WHITE,
            surface_alt: Rgb565::new(128, 128, 128),
            text: Rgb565::BLACK,
            text_secondary: Rgb565::BLUE,
            text_disable: Rgb565::new(64, 64, 64),
            border: Rgb565::BLACK,
            accent: Rgb565::BLUE,
            success: Rgb565::GREEN,
            warning: Rgb565::YELLOW,
            danger: Rgb565::RED,
            green: Rgb565::new(12, 46, 8),
            yello: Rgb565::new(31, 51, 0),
            orange: Rgb565::new(30, 37, 3),
            blue: Rgb565::new(0, 28, 23),
            purple: Rgb565::new(12, 11, 18),
        }
    }
}
