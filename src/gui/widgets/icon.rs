use embedded_graphics::{
    Drawable, Pixel,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::PixelColor,
    primitives::Rectangle,
};

use crate::gui::{
    event::{EventResult, GuiEvent},
    icons::IconData,
    widget::{Widget, WidgetBase},
};

pub struct Icon {
    base: WidgetBase,
    data: &'static IconData,
}

impl Icon {
    pub fn new(pos: Point, data: &'static IconData) -> Self {
        Self {
            base: WidgetBase::new(Rectangle::new(pos, Size::new(data.width, data.height))),
            data,
        }
    }
}

impl<D, C> Widget<D, C> for Icon
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    fn base(&self) -> &WidgetBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn draw(
        &self,
        ctx: &mut crate::gui::draw::DrawContext<D, C>,
    ) -> Result<(), <D as DrawTarget>::Error> {
        let c1 = ctx.theme().green();
        let c2 = ctx.theme().orange();
        let c3 = ctx.theme().purple();
        let c4 = ctx.theme().blue();

        for y in 0..self.data.height as usize {
            let mut bits = self.data.rows[y];

            for x in 0..self.data.width as usize {
                if (bits & 0x8000) != 0 {
                    let color = match (y as u32 >> 2) & 0b11 {
                        0 => c1,
                        1 => c2,
                        2 => c3,
                        _ => c4,
                    };

                    let p = ctx.point_to_screen(Point::new(x as i32, y as i32));
                    Pixel(p, color).draw(ctx.target())?;
                }

                bits <<= 1;
            }
        }
        Ok(())
    }

    fn event(&mut self, _event: &GuiEvent) -> EventResult {
        EventResult::Ignored
    }
}
