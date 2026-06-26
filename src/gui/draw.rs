use embedded_graphics::{
    Drawable, Pixel,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::PixelColor,
    primitives::{Line, Primitive, PrimitiveStyle, Rectangle},
};

use super::theme::Theme;

pub const ROUND6: [i32; 6] = [5, 4, 2, 1, 1, 0];

pub struct DrawContext<'a, D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    target: &'a mut D,
    theme: &'a Theme<C>,
    clip: Rectangle,
    dirty: Rectangle,
    focused: bool,
}

impl<'a, D, C> DrawContext<'a, D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    pub fn new(target: &'a mut D, theme: &'a Theme<C>) -> Self {
        Self {
            target,
            theme,
            clip: Rectangle {
                top_left: Point::new(0, 0),
                size: Size::new(0, 0),
            },
            dirty: Rectangle {
                top_left: Point::new(0, 0),
                size: Size::new(0, 0),
            },
            focused: false,
        }
    }
    pub fn target(&mut self) -> &mut D {
        self.target
    }

    pub fn theme(&self) -> &Theme<C> {
        self.theme
    }

    pub fn clip(&self) -> Rectangle {
        self.clip
    }

    pub fn dirty(&self) -> Rectangle {
        self.clip
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn fill_desktop(&mut self, rect: Rectangle) -> Result<(), D::Error> {
        let bg = self.theme().bg();
        let fg = self.theme().fg();

        rect.into_styled(PrimitiveStyle::with_fill(bg))
            .draw(self.target())?;
        for y in rect.top_left.y..rect.bottom_right().unwrap().y {
            for x in rect.top_left.x..rect.bottom_right().unwrap().x {
                if ((x + y) & 1) == 0 {
                    Pixel(Point::new(x, y), fg).draw(self.target())?;
                }
            }
        }

        Ok(())
    }

    pub fn fill_round_top_bar(&mut self, rect: Rectangle, color: C) -> Result<(), D::Error> {
        for (row, inset) in ROUND6.iter().enumerate() {
            let y = rect.top_left.y + row as i32;
            let x0 = rect.top_left.x + *inset;
            let x1 = rect.top_left.x + rect.size.width as i32 - *inset - 1;
            Line::new(Point::new(x0, y), Point::new(x1, y))
                .into_styled(PrimitiveStyle::with_stroke(color, 1))
                .draw(self.target())?;
        }
        Rectangle::new(
            Point::new(rect.top_left.x, rect.top_left.y + ROUND6.len() as i32),
            Size::new(rect.size.width, rect.size.height - ROUND6.len() as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(self.target())?;
        Ok(())
    }
}
