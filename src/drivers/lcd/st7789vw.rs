use crate::{
    drivers::{delay_ms, gpio, lcd::DisplayRoation, spi},
    sys::{
        device_driver::{self, DevError},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

enum Buffertype {
    Data,
    Command,
}

struct St7789vwLcdInner {
    spi: &'static dyn spi::interface::SpiBus,
    gpio: &'static dyn gpio::interface::Gpio,
    dc_pin: gpio::Pin,
    rst_pin: gpio::Pin,
    cs_pin: gpio::Pin,
    width: u32,
    height: u32,
    x_offset: u32,
    y_offset: u32,
}

impl St7789vwLcdInner {
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
            width: 0,
            height: 0,
            x_offset: 0,
            y_offset: 0,
        }
    }

    fn init(&self) -> Result<(), DevError> {
        self.hard_reset();

        self.soft_reset()?;

        // Pixel format
        self.send_buf(Buffertype::Command, &[0x3A])?;
        self.send_buf(Buffertype::Data, &[0x05])?;

        // Porch settings
        self.send_buf(Buffertype::Command, &[0xB2])?;
        self.send_buf(Buffertype::Data, &[0x0C, 0x0C, 0x00, 0x33, 0x33])?;

        // Gate control
        self.send_buf(Buffertype::Command, &[0xB7])?;
        self.send_buf(Buffertype::Data, &[0x35])?;

        // VCOM control
        self.send_buf(Buffertype::Command, &[0xBB])?;
        self.send_buf(Buffertype::Data, &[0x19])?;

        // LCM control
        self.send_buf(Buffertype::Command, &[0xC0])?;
        self.send_buf(Buffertype::Data, &[0x2C])?;

        // VDV and VRH command enable
        self.send_buf(Buffertype::Command, &[0xC2])?;
        self.send_buf(Buffertype::Data, &[0x01])?;
        self.send_buf(Buffertype::Command, &[0xC3])?;
        self.send_buf(Buffertype::Data, &[0x12])?;
        self.send_buf(Buffertype::Command, &[0xC4])?;
        self.send_buf(Buffertype::Data, &[0x20])?;

        // Frame rate
        self.send_buf(Buffertype::Command, &[0xC6])?;
        self.send_buf(Buffertype::Data, &[0x0F])?;

        // Power control
        self.send_buf(Buffertype::Command, &[0xD0])?;
        self.send_buf(Buffertype::Data, &[0xA4, 0xA1])?;

        // Gamma +
        self.send_buf(Buffertype::Command, &[0xE0])?;
        self.send_buf(
            Buffertype::Data,
            &[
                0xD0, 0x04, 0x0D, 0x11, 0x13, 0x2B, 0x3F, 0x54, 0x4C, 0x18, 0x0D, 0x0B, 0x1F, 0x23,
            ],
        )?;

        // Gamma -
        self.send_buf(Buffertype::Command, &[0xE1])?;
        self.send_buf(
            Buffertype::Data,
            &[
                0xD0, 0x04, 0x0C, 0x11, 0x13, 0x2C, 0x3F, 0x44, 0x51, 0x2F, 0x1F, 0x1F, 0x20, 0x23,
            ],
        )?;

        // Inversion
        self.send_buf(Buffertype::Command, &[0x21])?;

        self.sleep_out()?;

        Ok(())
    }

    fn config(&mut self, config: &super::LcdConfig) -> Result<(), DevError> {
        let madctl = match config.ration {
            DisplayRoation::Roation0 => 0x00,
            DisplayRoation::Roation90 => 0x70,
            DisplayRoation::Roation180 => 0x38,
            DisplayRoation::Roation270 => 0x68,
        };
        self.send_buf(Buffertype::Command, &[0x36])?;
        self.send_buf(Buffertype::Data, &[madctl])?;

        self.width = config.width;
        self.height = config.height;
        self.x_offset = config.x_offset;
        self.y_offset = config.y_offset;

        Ok(())
    }

    fn hard_reset(&self) {
        self.gpio.set_level(&self.rst_pin, gpio::Level::Low);
        delay_ms(10);
        self.gpio.set_level(&self.rst_pin, gpio::Level::High);
        delay_ms(120);
    }

    fn soft_reset(&self) -> Result<(), DevError> {
        self.send_buf(Buffertype::Command, &[0x01])?;
        delay_ms(150);
        Ok(())
    }

    fn sleep_out(&self) -> Result<(), DevError> {
        self.send_buf(Buffertype::Command, &[0x11])?;
        delay_ms(150);
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

        self.send_buf(Buffertype::Command, &[0x2A])?;
        self.send_buf(
            Buffertype::Data,
            &[(xs >> 8) as u8, xs as u8, (xe >> 8) as u8, xe as u8],
        )?;

        self.send_buf(Buffertype::Command, &[0x2B])?;
        self.send_buf(
            Buffertype::Data,
            &[(ys >> 8) as u8, ys as u8, (ye >> 8) as u8, ye as u8],
        )?;

        self.send_buf(Buffertype::Command, &[0x2C])?;

        Ok(())
    }

    fn flush_buf(&self, x: u16, y: u16, w: u16, h: u16, buf: &[u8]) -> Result<(), DevError> {
        let len = if buf.len() > (w as usize) * (h as usize) * 2 {
            (w as usize) * (h as usize) * 2
        } else {
            buf.len()
        };

        self.set_window(x, y, x + w, y + h)?;
        self.send_buf(Buffertype::Data, &buf[..len])
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
}

pub struct St7789vwLcd {
    inner: IrqSafeNullLock<St7789vwLcdInner>,
}

impl St7789vwLcd {
    pub const COMPATIBLE: &'static str = "ST7789VW LCD";

    pub const fn new(
        spi: &'static dyn crate::drivers::spi::interface::SpiBus,
        gpio: &'static dyn crate::drivers::gpio::interface::Gpio,
        dc_pin: usize,
        rst_pin: usize,
        cs_pin: usize,
    ) -> Self {
        Self {
            inner: IrqSafeNullLock::new(St7789vwLcdInner::new(spi, gpio, dc_pin, rst_pin, cs_pin)),
        }
    }
}

impl super::interface::Lcd for St7789vwLcd {
    fn config(&self, config: &super::LcdConfig) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.config(config))
    }

    fn display_on(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.display_on())
    }

    fn set_window(&self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.set_window(x0, y0, x1, y1))
    }

    fn flush_buf(&self, x: u16, y: u16, w: u16, h: u16, buf: &[u8]) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.flush_buf(x, y, w, h, buf))
    }
}

impl device_driver::interface::Driver for St7789vwLcd {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init())?;
        Ok(())
    }
}

impl device_driver::interface::Device for St7789vwLcd {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, data: &[u8]) -> Result<usize, device_driver::DevError> {
        if data.len() < 8 {
            return Err(device_driver::DevError::InvalidArg);
        }
        let x = u16::from_be_bytes([data[0], data[1]]);
        let y = u16::from_be_bytes([data[2], data[3]]);
        let w = u16::from_be_bytes([data[4], data[5]]);
        let h = u16::from_be_bytes([data[6], data[7]]);
        let addr = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let ptr = addr as *const u8;
        let len = w as usize * h as usize * 2;
        let payload = unsafe { core::slice::from_raw_parts(ptr, len) };

        self.inner
            .lock(|inner| inner.flush_buf(x, y, w, h, payload))
            .ok();
        Result::Ok(data.len())
    }
}

impl device_driver::interface::DeviceDriver for St7789vwLcd {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}
