use embedded_graphics::pixelcolor::PixelColor;

pub struct Theme<C>
where
    C: PixelColor,
{
    bg: C,
    fg: C,

    text: C,

    border: C,

    focus_bg: C,
    focus_fg: C,

    button_bg: C,
    button_fg: C,

    list_bg: C,
    list_fg: C,
}

impl<C: PixelColor> Theme<C> {
    pub fn classic(bg: C, fg: C, text: C, border: C) -> Self {
        Self {
            bg,
            fg,
            text,
            border,
            focus_bg: bg,
            focus_fg: fg,
            button_bg: bg,
            button_fg: fg,
            list_bg: bg,
            list_fg: fg,
        }
    }
    pub fn bg(&self) -> C {
        self.bg
    }

    pub fn fg(&self) -> C {
        self.fg
    }

    pub fn text(&self) -> C {
        self.text
    }

    pub fn border(&self) -> C {
        self.border
    }

    pub fn focus_bg(&self) -> C {
        self.focus_bg
    }

    pub fn focus_fg(&self) -> C {
        self.focus_fg
    }

    pub fn button_bg(&self) -> C {
        self.button_bg
    }

    pub fn button_fg(&self) -> C {
        self.button_fg
    }

    pub fn list_bg(&self) -> C {
        self.list_bg
    }

    pub fn list_fg(&self) -> C {
        self.list_fg
    }
}
