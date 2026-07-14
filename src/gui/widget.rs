use embedded_graphics::{geometry::Size, primitives::Rectangle};

use super::{
    draw::DrawContext,
    event::{EventResult, GuiEvent},
};

pub mod interface {
    use embedded_graphics::{
        draw_target::DrawTarget, pixelcolor::PixelColor, primitives::Rectangle,
    };

    pub trait Widget<D, C>
    where
        D: DrawTarget<Color = C>,
        C: PixelColor,
    {
        fn base(&self) -> &super::WidgetBase;

        fn base_mut(&mut self) -> &mut super::WidgetBase;

        fn rect(&self) -> Rectangle {
            self.base().rect()
        }

        fn set_rect(&mut self, rect: Rectangle) {
            self.base_mut().set_rect(rect);
        }

        fn draw(&self, ctx: &mut super::DrawContext<D, C>) -> Result<(), D::Error>;

        fn event(&mut self, event: &super::GuiEvent) -> super::EventResult;

        fn set_focus(&mut self, _focused: bool) {
            self.base_mut().focused = _focused;
        }

        fn is_focusable(&self) -> bool {
            false
        }

        fn visabel(&self) -> bool {
            self.base().visable
        }

        fn set_visable(&mut self, visable: bool) {
            self.base_mut().visable = visable;
        }
    }
}

pub struct WidgetBase {
    rect: Rectangle,
    visable: bool,
    focused: bool,
}

impl WidgetBase {
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            visable: true,
            focused: false,
        }
    }

    pub fn rect(&self) -> Rectangle {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rectangle) {
        self.rect = rect;
    }

    pub fn size(&self) -> Size {
        self.rect.size
    }
}
