use alloc::{boxed::Box, vec::Vec};

use embedded_graphics::{draw_target::DrawTarget, pixelcolor::PixelColor};

use super::{
    draw::DrawContext,
    event::{EventResult, GuiEvent},
    widget::interface::Widget,
};

pub struct Container<D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    children: Vec<Box<dyn Widget<D, C>>>,
}

impl<D, C> Container<D, C>
where
    D: DrawTarget<Color = C>,
    C: PixelColor,
{
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn add_child<W>(&mut self, child: W)
    where
        W: Widget<D, C> + 'static,
    {
        self.children.push(Box::new(child));
    }

    pub fn draw_children(&self, ctx: &mut DrawContext<D, C>) -> Result<(), D::Error> {
        for child in self.children.iter() {
            let child_rect = child.rect();
            ctx.with_origin(child_rect.top_left, |ctx| child.draw(ctx))?;
        }
        Ok(())
    }

    pub fn event_children(&mut self, event: &GuiEvent) -> EventResult {
        for child in self.children.iter_mut().rev() {
            if child.event(event).is_handled() {
                return EventResult::Consumed;
            }
        }

        EventResult::Ignored
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}
