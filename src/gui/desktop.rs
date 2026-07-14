use alloc::string::String;
use embedded_graphics::{
    draw_target::DrawTarget, geometry::Point, pixelcolor::PixelColor, primitives::Rectangle,
};

use super::{
    container::Container,
    draw::DrawContext,
    event::{EventResult, GuiEvent},
    widget::WidgetBase,
    widget::interface::Widget,
};

pub struct Desktop<D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    base: WidgetBase,
    title: String,
    container: Container<D, C>,
}

impl<D, C> Desktop<D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    pub fn new(rect: Rectangle) -> Self {
        Self {
            base: WidgetBase::new(rect),
            title: String::new(),
            container: Container::new(),
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

    pub fn add_child<W>(&mut self, child: W)
    where
        W: Widget<D, C> + 'static,
    {
        self.container.add_child(child);
    }
}

impl<D, C> Widget<D, C> for Desktop<D, C>
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
        ctx.with_origin(self.rect().top_left, |ctx| {
            let rect = ctx.rect_to_screen(Rectangle::new(Point::zero(), self.rect().size));

            ctx.fill_desktop(rect)?;

            self.container.draw_children(ctx)?;

            Ok(())
        })
    }

    fn event(&mut self, _event: &GuiEvent) -> EventResult {
        EventResult::Ignored
    }
}
