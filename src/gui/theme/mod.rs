use embedded_graphics::pixelcolor::PixelColor;

pub mod palettes;
pub struct Palette<C>
where
    C: PixelColor,
{
    pub background: C,
    pub surface: C,
    pub surface_alt: C,

    pub text: C,
    pub text_secondary: C,
    pub text_disable: C,

    pub border: C,
    pub accent: C,

    pub success: C,
    pub warning: C,
    pub danger: C,

    pub green: C,
    pub yello: C,
    pub orange: C,
    pub blue: C,
    pub purple: C,
}

pub struct DesktopStyle<C>
where
    C: PixelColor,
{
    pub background: C,
    pub app_label: C,
    pub app_label_background: C,
}

pub struct MenuBarStyle<C>
where
    C: PixelColor,
{
    pub background: C,
    pub text: C,
    pub border: C,
    pub height: u32,
}

pub struct LabelStyle<C>
where
    C: PixelColor,
{
    pub text: C,
    pub disabled_text: C,
    pub background: C,
}

pub struct ButtonVisualStyle<C>
where
    C: PixelColor,
{
    pub background: C,
    pub border: C,
    pub text: C,
}
pub struct ButtonStyle<C>
where
    C: PixelColor,
{
    pub normal: ButtonVisualStyle<C>,
    pub focused: ButtonVisualStyle<C>,
    pub pressed: ButtonVisualStyle<C>,
    pub disabled: ButtonVisualStyle<C>,

    pub border_width: u32,
    pub corner_radius: u32,
}

pub struct SysColor<C>
where
    C: PixelColor,
{
    pub green: C,
    pub yello: C,
    pub orange: C,
    pub blue: C,
    pub purple: C,
}

pub struct Theme<C>
where
    C: PixelColor,
{
    pub desktop: DesktopStyle<C>,
    pub menu_bar: MenuBarStyle<C>,
    pub label: LabelStyle<C>,
    pub sys_color: SysColor<C>,
}

pub struct ThemeBuilder<C>
where
    C: PixelColor,
{
    palette: Palette<C>,
}

impl<C> ThemeBuilder<C>
where
    C: PixelColor,
{
    pub fn new(palette: Palette<C>) -> Self {
        Self { palette }
    }

    pub fn build(self) -> Theme<C> {
        let p = self.palette;

        Theme {
            desktop: DesktopStyle {
                background: p.background,
                app_label: p.text_secondary,
                app_label_background: p.surface,
            },
            menu_bar: MenuBarStyle {
                background: p.surface,
                text: p.text,
                border: p.border,
                height: 22,
            },
            label: LabelStyle {
                text: p.text,
                disabled_text: p.text_disable,
                background: p.surface,
            },
            sys_color: SysColor {
                green: p.green,
                yello: p.yello,
                orange: p.orange,
                blue: p.blue,
                purple: p.purple,
            },
        }
    }
}
