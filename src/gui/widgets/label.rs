use alloc::string::String;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::PixelColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};

use super::super::{
    draw::DrawContext,
    event::{EventResult, GuiEvent},
    widget::{WidgetBase, interface::Widget},
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

    pub fn with_content(mut self, content: &str) -> Self {
        self.content.clear();
        let _ = self.content.push_str(content);
        self
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
        let local_rect = Rectangle::new(Point::zero(), self.base.rect().size);
        let rect = ctx.rect_to_screen(local_rect);
        rect.into_styled(PrimitiveStyle::with_fill(ctx.theme().label.background))
            .draw(ctx.target())?;
        let style = MonoTextStyle::new(&FONT_10X20, ctx.theme().label.text);

        Text::with_baseline(&self.content, rect.top_left, style, Baseline::Top)
            .draw(ctx.target())?;

        Ok(())
    }

    fn event(&mut self, _event: &GuiEvent) -> EventResult {
        EventResult::Ignored
    }
}
