use core::convert::Infallible;

use embedded_graphics::{
    Pixel,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{DrawTarget, OriginDimensions, RawData},
};

#[derive(Clone, Copy)]
struct DirtyRect {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
}

impl DirtyRect {
    fn include(&mut self, x: usize, y: usize) {
        self.x0 = self.x0.min(x);
        self.y0 = self.y0.min(y);
        self.x1 = self.x1.max(x);
        self.y1 = self.y1.max(y);
    }
}

pub struct FramebufferDisplay<'a, const W: usize, const H: usize, const PIXELS: usize> {
    lcd: &'a dyn super::interface::LcdFlush,
    fb: &'a mut [u16; PIXELS],
    dirty: Option<DirtyRect>,
}

#[allow(dead_code)]
impl<'a, const W: usize, const H: usize, const PIXELS: usize> FramebufferDisplay<'a, W, H, PIXELS> {
    pub fn new(lcd: &'a dyn super::interface::LcdFlush, fb: &'a mut [u16; PIXELS]) -> Self {
        Self {
            lcd,
            fb,
            dirty: None,
        }
    }

    pub fn clear_fb(&mut self, color: Rgb565) {
        let raw = RawU16::from(color).into_inner();

        for p in self.fb.iter_mut() {
            *p = raw;
        }

        self.dirty = Some(DirtyRect {
            x0: 0,
            y0: 0,
            x1: W - 1,
            y1: H - 1,
        });
    }

    pub fn flush(&mut self) {
        let Some(d) = self.dirty.take() else {
            return;
        };

        let w = d.x1 - d.x0 + 1;
        let h = d.y1 - d.y0 + 1;

        self.lcd
            .set_window(d.x0 as u16, d.y0 as u16, w as u16, h as u16);

        for y in d.y0..=d.y1 {
            let start = y * W + d.x0;
            let end = start + w;

            self.lcd.flush_buf_u16(&self.fb[start..end]);
        }
    }
}

impl<const W: usize, const H: usize, const PIXELS: usize> DrawTarget
    for FramebufferDisplay<'_, W, H, PIXELS>
{
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(p, color) in pixels {
            if p.x < 0 || p.y < 0 {
                continue;
            }

            let x = p.x as usize;
            let y = p.y as usize;

            if x >= W || y >= H {
                continue;
            }

            let idx = y * W + x;
            self.fb[idx] = RawU16::from(color).into_inner();

            match &mut self.dirty {
                Some(d) => d.include(x, y),
                None => {
                    self.dirty = Some(DirtyRect {
                        x0: x,
                        y0: y,
                        x1: x,
                        y1: y,
                    });
                }
            }
        }

        Ok(())
    }
}

impl<const W: usize, const H: usize, const PIXELS: usize> OriginDimensions
    for FramebufferDisplay<'_, W, H, PIXELS>
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        embedded_graphics::prelude::Size::new(W as u32, H as u32)
    }
}
