use crate::{
    drivers::i2c::interface::I2cBus,
    gui::input::{MAX_TOUCH_POINTS, TouchPoint, TouchReport, interface::TouchDevice},
    sys::{
        device_driver::{self, DevError, interface::Driver},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

const GT911_MAX_POINTS: usize = 5;
const POINT_SIZE: usize = 8;

const REG_STATUS: u16 = 0x814E;
const REG_POINT1: u16 = 0x814F;

pub struct Gt911Inner {
    i2c: &'static dyn I2cBus,
    addr: u8,
}

impl Gt911Inner {
    fn new(i2c: &'static dyn I2cBus, addr: u8) -> Self {
        Self { i2c, addr }
    }

    fn read_reg(&self, reg: u16, buf: &mut [u8]) -> Result<(), DevError> {
        self.i2c.write_read(self.addr, &reg.to_be_bytes(), buf)
    }

    fn write_reg(&self, reg: u16, data: &[u8]) -> Result<(), DevError> {
        let mut buf = [0u8; 8];

        if data.len() + 2 > buf.len() {
            return Err(DevError::InvalidArg);
        }

        buf[..2].copy_from_slice(&reg.to_be_bytes());
        buf[2..2 + data.len()].copy_from_slice(data);

        self.i2c.write(self.addr, &buf[..2 + data.len()])
    }

    fn clear_status(&self) -> Result<(), DevError> {
        self.write_reg(REG_STATUS, &[0x00])
    }

    fn read_point(&self) -> Result<Option<TouchReport>, DevError> {
        let mut status = [0u8; 1];
        self.read_reg(REG_STATUS, &mut status)?;

        let ready = status[0] & 0x80 != 0;
        let count = (status[0] & 0x0f) as usize;

        if !ready {
            return Ok(None);
        }

        if count == 0 {
            self.clear_status()?;
            return Ok(None);
        }

        let count = core::cmp::min(count, GT911_MAX_POINTS);

        let mut buf = [0u8; GT911_MAX_POINTS * POINT_SIZE];
        self.read_reg(REG_POINT1, &mut buf[..count * POINT_SIZE])?;

        self.clear_status()?;

        let mut points = [TouchPoint::EMPTY; MAX_TOUCH_POINTS];

        for i in 0..count {
            let base = i * POINT_SIZE;

            points[i] = TouchPoint {
                id: buf[base],
                x: u16::from_le_bytes([buf[base + 1], buf[base + 2]]),
                y: u16::from_le_bytes([buf[base + 3], buf[base + 4]]),
                size: u16::from_le_bytes([buf[base + 5], buf[base + 6]]),
            };
        }

        Ok(Some(TouchReport { count, points }))
    }
}

pub struct Gt911 {
    inner: IrqSafeNullLock<Gt911Inner>,
}

impl Gt911 {
    pub const COMPATIBLE: &'static str = "GT911 Touchpanel";

    pub const fn new(i2c: &'static dyn I2cBus, addr: u8) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Gt911Inner { i2c, addr }),
        }
    }
}

impl Driver for Gt911 {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        Ok(())
    }

    fn register_irq_handler(
        &'static self,
        _irq_number: Self::IrqNumberType,
    ) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<'a> device_driver::interface::Device for Gt911 {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }
}

impl<'a> device_driver::interface::DeviceDriver for Gt911 {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl TouchDevice for Gt911 {
    fn read_point(&self) -> Result<Option<crate::gui::input::TouchReport>, DevError> {
        self.inner.lock(|inner| inner.read_point())
    }
}
