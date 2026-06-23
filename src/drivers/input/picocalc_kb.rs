use crate::{
    drivers::{delay_ms, i2c},
    gui::input::interface::KeyboardDevice,
    sys::{
        device_driver,
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

const REG_ID_KEY: u8 = 0x04;
const REG_ID_FIF: u8 = 0x09;
const KEY_COUNT_MASK: u8 = 0x1f;

struct PicocalcKeyboardInner {
    i2c: &'static dyn i2c::interface::I2cBus,
    addr: u8,
}

impl PicocalcKeyboardInner {
    const fn new(i2c: &'static dyn i2c::interface::I2cBus, addr: u8) -> Self {
        Self { i2c, addr }
    }

    fn init(&self) -> Result<(), device_driver::DevError> {
        Ok(())
    }

    fn get_key_count(&self) -> Result<u8, device_driver::DevError> {
        self.i2c.write(self.addr, &[REG_ID_KEY])?;

        delay_ms(16);

        let mut status = [0u8; 2];
        self.i2c.read(self.addr, &mut status)?;

        Ok(status[0] & KEY_COUNT_MASK)
    }

    fn read_key_event(&self, data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        if data.len() != 2 {
            return Err(device_driver::DevError::InvalidArg);
        }
        self.i2c.write(self.addr, &[REG_ID_FIF])?;

        delay_ms(16);

        self.i2c.read(self.addr, data)
    }
}

pub struct PicocalcKeyboard {
    inner: IrqSafeNullLock<PicocalcKeyboardInner>,
}

impl PicocalcKeyboard {
    pub const COMPATIBLE: &'static str = "Picocalc Keyboayd";

    pub const fn new(i2c: &'static dyn i2c::interface::I2cBus, addr: u8) -> Self {
        Self {
            inner: IrqSafeNullLock::new(PicocalcKeyboardInner::new(i2c, addr)),
        }
    }
}

impl device_driver::interface::Driver for PicocalcKeyboard {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn init(&self) -> Result<(), device_driver::DevError> {
        self.inner.lock(|inner| inner.init())
    }

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }
}

impl device_driver::interface::Device for PicocalcKeyboard {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }
}

impl device_driver::interface::DeviceDriver for PicocalcKeyboard {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl KeyboardDevice for PicocalcKeyboard {
    fn read_key_value(
        &self,
    ) -> Result<Option<crate::gui::input::KeyValueReport>, device_driver::DevError> {
        self.inner.lock(|inner| {
            let count = inner.get_key_count()?;
            if count == 0 {
                return Ok(None);
            }

            let mut data = [0u8; 2];
            inner.read_key_event(&mut data)?;

            let key_rep = crate::gui::input::KeyValueReport::from_bytes(data);

            return Ok(Some(key_rep));
        })
    }
}
