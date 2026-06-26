use embedded_graphics::{draw_target::DrawTarget, pixelcolor::PixelColor, primitives::Rectangle};

use super::{
    draw::DrawContext,
    event::{EventResult, GuiEvent},
};

pub trait Widget<C>
where
    C: PixelColor,
{
    fn rect(&self) -> Rectangle;

    fn set_rect(&mut self, rect: Rectangle);

    fn draw<D>(&self, ctx: &mut DrawContext<D, C>) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = C>;

    fn event(&mut self, event: &GuiEvent) -> EventResult;

    fn set_focus(&mut self, _focused: bool) {}

    fn is_focusable(&self) -> bool {
        false
    }
}
