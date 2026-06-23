use crate::{
    drivers::{
        delay_ms,
        gpio::{self, Direction, Function, Pull},
        spi,
    },
    sys::{
        device_driver::{self, DevError},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

enum Buffertype {
    Data,
    Command,
}

// Command format: [Cmd] [Data] [delay_ms]
static ILI9488_INIT_CMDS: &[(&[u8], &[u8], u32)] = &[
    // Positive Gamma Control
    (
        &[0xE0],
        &[
            0x00, 0x03, 0x09, 0x08, 0x16, 0x0A, 0x3F, 0x78, 0x4C, 0x09, 0x0A, 0x08, 0x16, 0x1A,
            0x0F,
        ],
        0,
    ),
    // Negative Gamma Control
    (
        &[0xE1],
        &[
            0x00, 0x16, 0x19, 0x03, 0x0F, 0x05, 0x32, 0x45, 0x46, 0x04, 0x0E, 0x0D, 0x35, 0x37,
            0x0F,
        ],
        0,
    ),
    // Power Control 1
    (&[0xC0], &[0x17, 0x15], 0),
    // Power Control 2
    (&[0xC1], &[0x41], 0),
    // VCOM Control
    (&[0xC5], &[0x00, 0x12, 0x80], 0),
    // Memory Access Control: MX, BGR
    (&[0x36], &[0x48], 0),
    // Pixel Interface Format: 18-bit colour for SPI
    //(&[0x3A], &[0x66], 0),
    (&[0x3A], &[0x55], 0),
    // Interface Mode Control
    (&[0xB0], &[0x00], 0),
    // Frame Rate Control
    (&[0xB1], &[0xA0], 0),
    // Display Inversion ON
    (&[0x21], &[], 0),
    // Display Inversion Control
    (&[0xB4], &[0x02], 0),
    // Display Function Control
    (&[0xB6], &[0x02, 0x02, 0x3B], 0),
    // Entry Mode Set
    (&[0xB7], &[0xC6], 0),
    (&[0xE9], &[0x00], 0),
    // Adjust Control 3
    (&[0xF7], &[0xA9, 0x51, 0x2C, 0x82], 0),
    // Sleep Out
    (&[0x11], &[], 120),
    // Display ON
    (&[0x29], &[], 120),
    // MADCTL / orientation
    (&[0x36], &[0x48], 0),
];

struct Ili9488Inner<const W: usize, const H: usize> {
    spi: &'static dyn spi::interface::SpiBus,
    gpio: &'static dyn gpio::interface::Gpio,
    dc_pin: gpio::Pin,
    rst_pin: gpio::Pin,
    cs_pin: gpio::Pin,
    x_offset: usize,
    y_offset: usize,
}

impl<const W: usize, const H: usize> Ili9488Inner<W, H> {
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
        // Lcd pins
        // dc
        self.gpio.pin_config(
            self.dc_pin.0,
            Function::SIO,
            Pull::None,
            Some(Direction::Output),
        );
        // cs
        self.gpio.pin_config(
            self.cs_pin.0,
            Function::SIO,
            Pull::None,
            Some(Direction::Output),
        );
        // rst
        self.gpio.pin_config(
            self.rst_pin.0,
            Function::SIO,
            Pull::None,
            Some(Direction::Output),
        );

        self.hard_reset();

        self.soft_reset()?;

        for (cmd, data, delay) in ILI9488_INIT_CMDS {
            defmt::info!("send cmd");
            self.send_cmd(cmd, data)?;
            delay_ms(*delay);
        }

        Ok(())
    }

    fn config(&mut self, _config: &super::LcdConfig) -> Result<(), DevError> {
        Ok(())
    }

    fn display_on(&self) -> Result<(), DevError> {
        self.send_buf(Buffertype::Command, &[0x29])?;
        delay_ms(20);
        Ok(())
    }

    fn set_window(&self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DevError> {
        let x0 = x0 + self.x_offset as u16;
        let x1 = x1 - 1 + self.x_offset as u16;
        let y0 = y0 + self.y_offset as u16;
        let y1 = y1 - 1 + self.y_offset as u16;

        self.send_cmd(
            &[0x2A],
            &[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8],
        )?;

        self.send_cmd(
            &[0x2B],
            &[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8],
        )?;

        self.send_buf(Buffertype::Command, &[0x2C])?;

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

pub struct Ili9488Lcd<const W: usize, const H: usize> {
    inner: IrqSafeNullLock<Ili9488Inner<W, H>>,
}

impl<const W: usize, const H: usize> Ili9488Lcd<W, H> {
    pub const COMPATIBLE: &'static str = "ILI9488 LCD";

    pub const fn new(
        spi: &'static dyn crate::drivers::spi::interface::SpiBus,
        gpio: &'static dyn crate::drivers::gpio::interface::Gpio,
        dc_pin: usize,
        rst_pin: usize,
        cs_pin: usize,
    ) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Ili9488Inner::new(spi, gpio, dc_pin, rst_pin, cs_pin)),
        }
    }
}

impl<const W: usize, const H: usize> super::interface::Lcd for Ili9488Lcd<W, H> {
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

impl<const W: usize, const H: usize> device_driver::interface::Driver for Ili9488Lcd<W, H> {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init())?;
        Ok(())
    }
}

impl<const W: usize, const H: usize> device_driver::interface::Device for Ili9488Lcd<W, H> {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        Ok(0)
    }
}

impl<const W: usize, const H: usize> device_driver::interface::DeviceDriver for Ili9488Lcd<W, H> {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl<const W: usize, const H: usize> crate::gui::interface::LcdFlush for Ili9488Lcd<W, H> {
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
