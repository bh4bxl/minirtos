use crate::{
    bsp::pac,
    drivers::i2c::I2cConfig,
    sys::{
        device_driver::{self, DevError},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum I2cId {
    I2C0,
    I2C1,
}

struct Rp235xI2cInner {
    id: I2cId,
    regs: *const pac::i2c0::RegisterBlock,
}

impl Rp235xI2cInner {
    const fn new(id: I2cId) -> Self {
        let regs = match id {
            I2cId::I2C0 => pac::I2C0::ptr(),
            I2cId::I2C1 => pac::I2C1::ptr(),
        };

        Self { id, regs }
    }

    fn regs(&self) -> &pac::i2c0::RegisterBlock {
        unsafe { &*self.regs }
    }

    fn init(&self) {
        let resets = unsafe { &*pac::RESETS::ptr() };

        match self.id {
            I2cId::I2C0 => {
                resets.reset().modify(|_, w| w.i2c0().set_bit());
                resets.reset().modify(|_, w| w.i2c0().clear_bit());
                while resets.reset_done().read().i2c0().bit_is_clear() {}
            }
            I2cId::I2C1 => {
                resets.reset().modify(|_, w| w.i2c1().set_bit());
                resets.reset().modify(|_, w| w.i2c1().clear_bit());
                while resets.reset_done().read().i2c1().bit_is_clear() {}
            }
        }
    }

    fn config(&self, cfg: &I2cConfig) {
        let regs = self.regs();

        // Disable controller
        regs.ic_enable().write(|w| w.enable().disabled());

        // Standard/Fast mode, master mode
        regs.ic_con().write(|w| {
            w.master_mode().enabled();
            w.ic_slave_disable().slave_disabled();
            w.ic_restart_en().enabled();
            w.speed().fast();
            w
        });

        // 7-bit address mode
        regs.ic_tar().write(|w| unsafe { w.ic_tar().bits(0) });

        // Timing, rough first version.
        // For 100k/400k this is usually enough to start.
        let period = cfg.clk_sys / cfg.baudrate;
        let high = period / 2;
        let low = period - high;

        regs.ic_fs_scl_hcnt()
            .write(|w| unsafe { w.ic_fs_scl_hcnt().bits(high as u16) });
        regs.ic_fs_scl_lcnt()
            .write(|w| unsafe { w.ic_fs_scl_lcnt().bits(low as u16) });

        // Enable controller
        regs.ic_enable().write(|w| w.enable().enabled());
    }

    fn wait_tx_not_full(&self) {
        while self.regs().ic_status().read().tfnf().bit_is_clear() {}
    }

    fn wait_rx_not_empty(&self) -> Result<(), DevError> {
        let regs = self.regs();

        let mut timeout = 100_000;

        while regs.ic_status().read().rfne().bit_is_clear() {
            if regs.ic_raw_intr_stat().read().tx_abrt().bit_is_set() {
                return Err(DevError::Io);
            }

            timeout -= 1;
            if timeout == 0 {
                return Err(DevError::Timeout);
            }
        }

        Ok(())
    }

    fn wait_idle(&self) {
        let regs = self.regs();

        while regs.ic_status().read().activity().bit_is_set() {}
    }

    fn set_target(&self, addr: u8) {
        let regs = self.regs();

        // Target address can only be changed safely when disabled.
        regs.ic_enable().write(|w| w.enable().disabled());
        while regs.ic_enable_status().read().ic_en().bit_is_set() {}

        regs.ic_tar()
            .write(|w| unsafe { w.ic_tar().bits(addr as u16) });

        regs.ic_enable().write(|w| w.enable().enabled());
    }

    fn write(&self, addr: u8, data: &[u8]) -> Result<(), DevError> {
        if data.is_empty() {
            return Ok(());
        }

        self.set_target(addr);

        for (i, &b) in data.iter().enumerate() {
            self.wait_tx_not_full();

            let is_last = i == data.len() - 1;

            self.regs().ic_data_cmd().write(|w| unsafe {
                w.dat().bits(b);
                w.cmd().write();
                w.stop().bit(is_last);
                w
            });
        }

        self.wait_idle();

        Ok(())
    }

    fn read(&self, addr: u8, data: &mut [u8]) -> Result<usize, DevError> {
        if data.is_empty() {
            return Ok(0);
        }

        self.set_target(addr);

        for i in 0..data.len() {
            self.wait_tx_not_full();

            let is_last = i == data.len() - 1;

            self.regs().ic_data_cmd().write(|w| {
                w.cmd().read();
                w.stop().bit(is_last);
                w
            });
        }

        for b in data.iter_mut() {
            self.wait_rx_not_empty()?;
            *b = self.regs().ic_data_cmd().read().dat().bits();
        }

        self.wait_idle();

        Ok(data.len())
    }

    fn write_read(&self, addr: u8, write: &[u8], read: &mut [u8]) -> Result<(), DevError> {
        self.set_target(addr);

        for &b in write {
            self.wait_tx_not_full();

            self.regs().ic_data_cmd().write(|w| unsafe {
                w.dat().bits(b);
                w.cmd().write();
                w.restart().clear_bit();
                w.stop().clear_bit();
                w
            });
        }

        for i in 0..read.len() {
            self.wait_tx_not_full();

            let is_last = i == read.len() - 1;

            self.regs().ic_data_cmd().write(|w| {
                w.cmd().read();
                w.restart().bit(i == 0);
                w.stop().bit(is_last);
                w
            });
        }

        for b in read.iter_mut() {
            self.wait_rx_not_empty()?;
            *b = self.regs().ic_data_cmd().read().dat().bits();
        }

        self.wait_idle();

        Ok(())
    }
}

pub struct Rp235xI2c {
    inner: IrqSafeNullLock<Rp235xI2cInner>,
}

impl Rp235xI2c {
    pub const COMPATIBLE: &'static str = "RP235x I2C";

    pub const fn new(id: I2cId) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Rp235xI2cInner::new(id)),
        }
    }

    pub fn config(&self, cfg: &I2cConfig) {
        self.inner.lock(|inner| inner.config(cfg));
    }
}

impl super::interface::I2cBus for Rp235xI2c {
    fn write(&self, addr: u8, data: &[u8]) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.write(addr, data))
    }

    fn read(&self, addr: u8, data: &mut [u8]) -> Result<usize, DevError> {
        self.inner.lock(|inner| inner.read(addr, data))
    }

    fn write_read(&self, addr: u8, write: &[u8], read: &mut [u8]) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.write_read(addr, write, read))
    }
}

impl device_driver::interface::Driver for Rp235xI2c {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init());
        Ok(())
    }
}

impl device_driver::interface::Device for Rp235xI2c {
    fn read(&self, data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        if data.len() < 2 {
            return Err(DevError::InvalidArg);
        }
        let addr = data[0];
        self.inner.lock(|inner| inner.read(addr, &mut data[1..]))
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }
}

impl device_driver::interface::DeviceDriver for Rp235xI2c {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}
