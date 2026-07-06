use embedded_graphics::pixelcolor::PixelColor;

pub struct Theme<C>
where
    C: PixelColor,
{
    bg: C,
    fg: C,

    text: C,

    border: C,

    green: C,
    yellow: C,
    orange: C,
    red: C,
    purple: C,
    blue: C,
}

impl<C: PixelColor> Theme<C> {
    pub fn classic(
        bg: C,
        fg: C,
        text: C,
        border: C,
        green: C,
        yellow: C,
        orange: C,
        red: C,
        purple: C,
        blue: C,
    ) -> Self {
        Self {
            bg,
            fg,
            text,
            border,
            green,
            yellow,
            orange,
            red,
            purple,
            blue,
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

    pub fn green(&self) -> C {
        self.green
    }

    pub fn yellow(&self) -> C {
        self.yellow
    }

    pub fn orange(&self) -> C {
        self.orange
    }

    pub fn red(&self) -> C {
        self.red
    }

    pub fn purple(&self) -> C {
        self.purple
    }

    pub fn blue(&self) -> C {
        self.blue
    }
}
