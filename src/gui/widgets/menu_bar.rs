use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::PixelColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
};

use super::super::{
    container::Container,
    event::{EventResult, GuiEvent},
    widget::{Widget, WidgetBase},
};

pub struct MenuBar<D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    base: WidgetBase,
    border: u32,
    container: Container<D, C>,
}

impl<D, C> MenuBar<D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    pub fn new(rect: Rectangle, border: u32) -> Self {
        Self {
            base: WidgetBase::new(rect),
            border,
            container: Container::new(),
        }
    }

    pub fn add_child<W>(&mut self, child: W)
    where
        W: Widget<D, C> + 'static,
    {
        self.container.add_child(child);
    }
}

impl<D, C> Widget<D, C> for MenuBar<D, C>
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
        let bg = ctx.theme().bg();
        let fg = ctx.theme().fg();
        let size = self.rect().size;
        // menu bar background
        let bg_rect = ctx.rect_to_screen(Rectangle::new(Point::zero(), size));
        bg_rect
            .into_styled(PrimitiveStyle::with_fill(bg))
            .draw(ctx.target())?;

        // menu bar
        let menu_bar = ctx.rect_to_screen(Rectangle::new(
            Point::zero(),
            Size::new(size.width, size.height.saturating_sub(self.border)),
        ));

        ctx.fill_round_top_bar(menu_bar, fg)?;

        self.container.draw_children(ctx)?;

        Ok(())
    }

    fn event(&mut self, _event: &GuiEvent) -> EventResult {
        EventResult::Ignored
    }
}
