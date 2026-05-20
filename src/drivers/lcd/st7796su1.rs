use crate::{
    drivers::{delay_ms, gpio, spi},
    sys::{
        device_driver::{self, DevError},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

use super::DisplayRoation;

enum Buffertype {
    Data,
    Command,
}

/// Command format: [Cmd] [Data] [delay_ms]
static ST7796SU1_INIT_CMDS: &[(&[u8], &[u8], u32)] = &[
    // Power control B
    (&[0xcf], &[0x00, 0x83, 0x30], 0),
    // Power on sequence control
    (&[0xed], &[0x64, 0x03, 0x12, 0x81], 0),
    // Driver timing control A
    (&[0xe8], &[0x85, 0x01, 0x79], 0),
    // Power control A
    (&[0xcb], &[0x39, 0x2C, 0x00, 0x34, 0x02], 0),
    // Pump ratio control
    (&[0xf7], &[0x20], 0),
    // Driver timing control B
    (&[0xea], &[0x00, 0x00], 0),
    // Power control 1
    (&[0xc0], &[0x26], 0),
    // Power control 2
    (&[0xc1], &[0x11], 0),
    // VCOM control 1
    (&[0xc5], &[0x35, 0x3e], 0),
    // VCOM control 2
    (&[0xc7], &[0xbe], 0),
    // Memory access control
    // 0x28:
    //   MV=1  (row/column exchange)
    //   BGR=1
    (&[0x36], &[0x28], 0),
    // Interface pixel format
    // 0x05 = RGB565
    (&[0x3a], &[0x05], 0),
    // Frame rate control
    (&[0xb1], &[0x00, 0x1b], 0),
    // 3Gamma function disable
    (&[0xf2], &[0x08], 0),
    // Gamma curve selected
    (&[0x26], &[0x01], 0),
    // Positive gamma correction
    (
        &[0xe0],
        &[
            0x1f, 0x1a, 0x18, 0x0a, 0x0f, 0x06, 0x45, 0x87, 0x32, 0x0a, 0x07, 0x02, 0x07, 0x05,
            0x00,
        ],
        0,
    ),
    // Negative gamma correction
    (
        &[0xe1],
        &[
            0x00, 0x25, 0x27, 0x05, 0x10, 0x09, 0x3a, 0x78, 0x4D, 0x05, 0x18, 0x0D, 0x38, 0x3a,
            0x1f,
        ],
        0,
    ),
    // Inversion ON
    (&[0x21], &[], 0),
    // Entry mode set
    (&[0xb7], &[0x07], 0),
    // Display function control
    (&[0xb6], &[0x0A, 0x82, 0x27, 0x00], 0),
    // Sleep out
    // Datasheet typically requires 120ms delay
    (&[0x11], &[0x00], 120),
];

pub struct St7796su1LcdInner<const W: usize, const H: usize> {
    spi: &'static dyn spi::interface::SpiBus,
    gpio: &'static dyn gpio::interface::Gpio,
    dc_pin: gpio::Pin,
    rst_pin: gpio::Pin,
    cs_pin: gpio::Pin,
    x_offset: usize,
    y_offset: usize,
}

impl<const W: usize, const H: usize> St7796su1LcdInner<W, H> {
    const fn new(
        spi: &'static dyn spi::interface::SpiBus,
        gpio: &'static dyn gpio::interface::Gpio,
        dc_pin: usize,
        rst_pin: usize,
        cs_pin: usize,
    ) -> Self {
        Self {
            spi,
            gpio,
            dc_pin: crate::drivers::gpio::Pin(dc_pin),
            rst_pin: crate::drivers::gpio::Pin(rst_pin),
            cs_pin: crate::drivers::gpio::Pin(cs_pin),
            x_offset: 0,
            y_offset: 0,
        }
    }

    fn hard_reset(&self) {
        self.gpio.set_level(&self.rst_pin, gpio::Level::Low);
        delay_ms(10);
        self.gpio.set_level(&self.rst_pin, gpio::Level::High);
        delay_ms(120);
    }

    fn send_buf(&self, buf_type: Buffertype, buf: &[u8]) -> Result<(), DevError> {
        match buf_type {
            Buffertype::Data => self.gpio.set_level(&self.dc_pin, gpio::Level::High),
            Buffertype::Command => self.gpio.set_level(&self.dc_pin, gpio::Level::Low),
        }

        self.gpio.set_level(&self.cs_pin, gpio::Level::Low);

        self.spi.write(buf)?;

        self.gpio.set_level(&self.cs_pin, gpio::Level::High);

        Ok(())
    }

    fn send_cmd(&self, cmd: &[u8], data: &[u8]) -> Result<(), DevError> {
        self.send_buf(Buffertype::Command, cmd)?;
        if data.len() > 0 {
            self.send_buf(Buffertype::Data, data)?;
        }
        Ok(())
    }

    fn soft_reset(&self) -> Result<(), DevError> {
        self.send_buf(Buffertype::Command, &[0x01])?;
        delay_ms(150);
        Ok(())
    }

    fn init(&self) -> Result<(), DevError> {
        self.hard_reset();

        self.soft_reset()?;

        for (cmd, data, delay) in ST7796SU1_INIT_CMDS {
            self.send_cmd(cmd, data)?;
            delay_ms(*delay);
        }

        Ok(())
    }

    fn config(&mut self, config: &super::LcdConfig) -> Result<(), DevError> {
        let madctl = match config.ration {
            DisplayRoation::Roation0 => 0x48,
            DisplayRoation::Roation90 => 0x28,
            DisplayRoation::Roation180 => 0x88,
            DisplayRoation::Roation270 => 0xe8,
        };
        self.send_cmd(&[0x36], &[madctl])?;

        self.x_offset = config.x_offset;
        self.y_offset = config.y_offset;

        Ok(())
    }

    fn display_on(&self) -> Result<(), DevError> {
        self.send_buf(Buffertype::Command, &[0x29])?;
        delay_ms(20);
        Ok(())
    }

    fn set_window(&self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DevError> {
        let x_offset = self.x_offset as u16;
        let y_offset = self.y_offset as u16;

        let xs = x0 + x_offset;
        let xe = x1 - 1 + x_offset;
        let ys = y0 + y_offset;
        let ye = y1 - 1 + y_offset;

        self.send_cmd(
            &[0x2a],
            &[(xs >> 8) as u8, xs as u8, (xe >> 8) as u8, xe as u8],
        )?;

        self.send_cmd(
            &[0x2b],
            &[(ys >> 8) as u8, ys as u8, (ye >> 8) as u8, ye as u8],
        )?;

        self.send_buf(Buffertype::Command, &[0x2c])?;

        Ok(())
    }

    fn flush_buf(&self, buf: &[u8]) -> Result<(), DevError> {
        self.send_buf(Buffertype::Data, buf)
    }

    fn flush_buf_dma(&self, buf: &[u8]) -> Result<(), DevError> {
        self.gpio.set_level(&self.dc_pin, gpio::Level::High);
        self.gpio.set_level(&self.cs_pin, gpio::Level::Low);

        self.spi.write_dma(buf)?;

        self.gpio.set_level(&self.cs_pin, gpio::Level::High);

        Ok(())
    }

    fn flush_buf_dma_u16(&self, buf: &[u16]) -> Result<(), DevError> {
        self.gpio.set_level(&self.dc_pin, gpio::Level::High);
        self.gpio.set_level(&self.cs_pin, gpio::Level::Low);

        self.spi.write_dma_u16(buf)?;

        self.gpio.set_level(&self.cs_pin, gpio::Level::High);

        Ok(())
    }
}

pub struct St7796su1Lcd<const W: usize, const H: usize> {
    inner: IrqSafeNullLock<St7796su1LcdInner<W, H>>,
}

impl<const W: usize, const H: usize> St7796su1Lcd<W, H> {
    pub const COMPATIBLE: &'static str = "ST7796SU1 LCD";

    pub const fn new(
        spi: &'static dyn crate::drivers::spi::interface::SpiBus,
        gpio: &'static dyn crate::drivers::gpio::interface::Gpio,
        dc_pin: usize,
        rst_pin: usize,
        cs_pin: usize,
    ) -> Self {
        Self {
            inner: IrqSafeNullLock::new(St7796su1LcdInner::new(spi, gpio, dc_pin, rst_pin, cs_pin)),
        }
    }
}

impl<const W: usize, const H: usize> super::interface::Lcd for St7796su1Lcd<W, H> {
    fn config(&self, config: &super::LcdConfig) -> Result<(), crate::sys::device_driver::DevError> {
        self.inner.lock(|inner| inner.config(config))
    }

    fn display_on(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.display_on())
    }

    fn set_window(&self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.set_window(x0, y0, x1, y1))
    }

    fn flush_buf(&self, buf: &[u8]) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.flush_buf(buf))
    }
}

impl<const W: usize, const H: usize> device_driver::interface::Driver for St7796su1Lcd<W, H> {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init())?;
        Ok(())
    }
}

impl<const W: usize, const H: usize> device_driver::interface::Device for St7796su1Lcd<W, H> {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        Ok(0)
    }
}

impl<const W: usize, const H: usize> device_driver::interface::DeviceDriver for St7796su1Lcd<W, H> {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl<const W: usize, const H: usize> crate::gui::interface::LcdFlush for St7796su1Lcd<W, H> {
    fn set_window(&self, x: u16, y: u16, w: u16, h: u16) {
        self.inner
            .lock(|inner| inner.set_window(x, y, x + w, y + h))
            .ok();
    }

    fn flush_buf(&self, data: &[u8]) {
        self.inner.lock(|inner| inner.flush_buf_dma(data)).ok();
    }

    fn flush_buf_u16(&self, data: &[u16]) {
        self.inner.lock(|inner| inner.flush_buf_dma_u16(data)).ok();
    }
}
