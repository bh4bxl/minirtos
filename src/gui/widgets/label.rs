use alloc::string::String;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    pixelcolor::PixelColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
};

use super::super::{
    draw::DrawContext,
    event::{EventResult, GuiEvent},
    widget::{Widget, WidgetBase},
};

pub struct Label {
    base: WidgetBase,
    content: String,
}

impl Label {
    pub fn new(rect: Rectangle) -> Self {
        Self {
            base: WidgetBase::new(rect),
            content: String::new(),
        }
    }
}

impl<D, C> Widget<D, C> for Label
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

    fn draw(&self, ctx: &mut DrawContext<D, C>) -> Result<(), D::Error> {
        let bg = ctx.theme().orange();

        let local_rect = Rectangle::new(Point::zero(), self.base.rect().size);
        let rect = ctx.rect_to_screen(local_rect);
        rect.into_styled(PrimitiveStyle::with_fill(bg))
            .draw(ctx.target())?;
        Ok(())
    }

    fn event(&mut self, _event: &GuiEvent) -> EventResult {
        EventResult::Ignored
    }
}
