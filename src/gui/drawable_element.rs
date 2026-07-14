use embedded_graphics::{
    Drawable, Pixel,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::PixelColor,
};

use crate::gui::icons::IconData;

pub mod interface {
    use embedded_graphics::{
        draw_target::DrawTarget,
        geometry::{Point, Size},
        pixelcolor::PixelColor,
    };

    use crate::gui::draw::DrawContext;

    pub trait DrawableElement<D, C>
    where
        D: DrawTarget<Color = C>,
        C: PixelColor,
    {
        fn size(&self) -> Size;

        fn draw(&self, ctx: &mut DrawContext<D, C>, pos: Point) -> Result<(), D::Error>;

        fn preferred_size(&self) -> Size {
            self.size()
        }
    }
}

pub struct Icon {
    data: &'static IconData,
}

impl Icon {
    pub fn new(data: &'static IconData) -> Self {
        Self { data }
    }
}

impl<D, C> interface::DrawableElement<D, C> for Icon
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        Size::new(self.data.width, self.data.height)
    }

    fn draw(
        &self,
        ctx: &mut super::draw::DrawContext<D, C>,
        pos: embedded_graphics::prelude::Point,
    ) -> Result<(), <D as DrawTarget>::Error> {
        let c1 = ctx.theme().sys_color.green;
        let c2 = ctx.theme().sys_color.orange;
        let c3 = ctx.theme().sys_color.purple;
        let c4 = ctx.theme().sys_color.blue;

        let stride = (self.data.width / 16) as usize;

        for y in 0..self.data.height as usize {
            for x in 0..self.data.width as usize {
                let word_idx = y * stride + x / 16;
                let bit_idx = x % 16;

                let bits = self.data.rows[word_idx];

                if (bits & (0x8000 >> bit_idx)) != 0 {
                    let color = match (y as u32 >> 2) & 0b11 {
                        0 => c1,
                        1 => c2,
                        2 => c3,
                        _ => c4,
                    };

                    let p = ctx.point_to_screen(pos + Point::new(x as i32, y as i32));
                    Pixel(p, color).draw(ctx.target())?;
                }
            }
        }

        Ok(())
    }
}
