use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::PixelColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

use crate::gui::{
    drawable_element::{Icon, interface::DrawableElement},
    event::{EventResult, GuiEvent},
    icons::IconData,
    widget::{WidgetBase, interface::Widget},
};

pub struct DesktopIcon {
    base: WidgetBase,
    icon: Icon,
    title: &'static str,
    app_id: usize,
}

impl DesktopIcon {
    pub fn new(pos: Point, data: &'static IconData, title: &'static str, app_id: usize) -> Self {
        let width = 56;
        let height = data.height + 16;

        Self {
            base: WidgetBase::new(Rectangle::new(pos, Size::new(width, height))),
            icon: Icon::new(data),
            title,
            app_id,
        }
    }

    pub fn app_id(&self) -> usize {
        self.app_id
    }
}

impl<D, C> Widget<D, C> for DesktopIcon
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
        let icon_size = <Icon as DrawableElement<D, C>>::size(&self.icon);
        let icon_x = (self.base.rect().size.width as i32 - icon_size.width as i32) / 2;
        self.icon.draw(ctx, Point::new(icon_x, 0))?;

        let label_y = icon_size.height as i32 + 4;

        let label_rect = Rectangle::new(
            ctx.point_to_screen(Point::new(0, label_y)),
            Size::new(self.base.rect().size.width, 20),
        );

        label_rect
            .into_styled(PrimitiveStyle::with_fill(
                ctx.theme().desktop.app_label_background,
            ))
            .draw(ctx.target())?;

        let text_style = MonoTextStyle::new(&FONT_10X20, ctx.theme().desktop.app_label);
        let text_pos = ctx.point_to_screen(Point::new(
            self.base.rect().size.width as i32 / 2,
            icon_size.height as i32 + 20,
        ));
        Text::with_alignment(self.title, text_pos, text_style, Alignment::Center)
            .draw(ctx.target())?;
        Ok(())
    }

    fn event(&mut self, event: &crate::gui::event::GuiEvent) -> crate::gui::event::EventResult {
        match event {
            GuiEvent::TouchUp(e) => {
                let p = Point::new(e.pos.x, e.pos.y);
                if self.base.rect().contains(p) {
                    EventResult::LaunchApp(self.app_id)
                } else {
                    EventResult::Ignored
                }
            }
            _ => EventResult::Ignored,
        }
    }
}
