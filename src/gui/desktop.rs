use embedded_graphics::{
    Drawable, Pixel,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::PixelColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
};
use heapless::String;

use super::{
    draw::DrawContext,
    event::{EventResult, GuiEvent},
    widget::Widget,
};

const MENU_BAR_HEIGHT: u32 = 22;
const MENU_BAR_BORDER: u32 = 2;
const ICON_AREA_WIDTH: u32 = 34;

const M_ICON: [u16; 16] = [
    0b1100110011001100,
    0b0000000000000000,
    0b1101111001111000,
    0b1111011111101110,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b1110000110000111,
    0b0000000000000000,
    0b0011001100110011,
];

pub struct Desktop<const N: usize> {
    rect: Rectangle,
    title: String<N>,
}

impl<const N: usize> Desktop<N> {
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            title: String::new(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title.clear();
        let _ = self.title.push_str(title);
        self
    }

    pub fn set_title(&mut self, title: &str) {
        self.title.clear();
        let _ = self.title.push_str(title);
    }

    fn draw_menu_bar<D, C>(&self, ctx: &mut DrawContext<D, C>) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = C>,
        C: PixelColor,
    {
        let bg = ctx.theme().bg();
        let fg = ctx.theme().fg();
        let text = ctx.theme().text();

        // menu bar background
        Rectangle::new(
            self.rect.top_left,
            Size::new(self.rect.size.width, MENU_BAR_HEIGHT + MENU_BAR_BORDER),
        )
        .into_styled(PrimitiveStyle::with_fill(bg))
        .draw(ctx.target())?;

        // menu bar
        let menu_bar = Rectangle::new(
            self.rect.top_left,
            Size::new(self.rect.size.width, MENU_BAR_HEIGHT),
        );

        ctx.fill_round_top_bar(menu_bar, fg)?;

        // icon
        for (y, row) in M_ICON.iter().enumerate() {
            let mut bits = *row;
            for x in 0..16 {
                if (bits & 0x8000) != 0 {
                    Pixel(
                        Point::new(
                            self.rect.top_left.x + 10 + x as i32,
                            self.rect.top_left.y + 3 + y as i32,
                        ),
                        text,
                    )
                    .draw(ctx.target())?;
                }
                bits <<= 1;
            }
        }

        Ok(())
    }
}

impl<C, const N: usize> Widget<C> for Desktop<N>
where
    C: PixelColor,
{
    fn rect(&self) -> Rectangle {
        self.rect
    }

    fn set_rect(&mut self, rect: Rectangle) {
        self.rect = rect;
    }

    fn draw<D>(&self, ctx: &mut DrawContext<D, C>) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = C>,
    {
        ctx.fill_desktop(self.rect)?;

        self.draw_menu_bar(ctx)?;

        Ok(())
    }

    fn event(&mut self, _event: &GuiEvent) -> EventResult {
        EventResult::Ignored
    }
}
