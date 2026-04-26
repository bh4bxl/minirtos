use crate::{
    bsp::pac,
    drivers::spi::{SpiConfig, SpiMode, interface},
    sys::{
        device_driver,
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SpiId {
    SPI0,
    SPI1,
}

struct Pl022SpiInner {
    id: SpiId,
    regs: *const pac::spi0::RegisterBlock,
}

#[allow(dead_code)]
impl Pl022SpiInner {
    /// Create an instance
    const fn new(id: SpiId) -> Self {
        let regs = match id {
            SpiId::SPI0 => unsafe { &*pac::SPI0::ptr() },
            SpiId::SPI1 => unsafe { &*pac::SPI1::ptr() },
        };

        Self { id, regs }
    }

    fn id(&self) -> SpiId {
        self.id
    }

    fn regs(&self) -> &pac::spi0::RegisterBlock {
        unsafe { &*self.regs }
    }

    fn init(&self) {
        let resets = unsafe { &*pac::RESETS::ptr() };
        match self.id {
            SpiId::SPI0 => {
                resets.reset().modify(|_, w| w.spi0().set_bit());
                resets.reset().modify(|_, w| w.spi0().clear_bit());

                while resets.reset_done().read().spi1().bit_is_clear() {}
            }
            SpiId::SPI1 => {
                resets.reset().modify(|_, w| w.spi1().set_bit());
                resets.reset().modify(|_, w| w.spi1().clear_bit());

                while resets.reset_done().read().spi1().bit_is_clear() {}
            }
        }
    }

    fn config(&self, config: &SpiConfig) {
        // Turn off SPI
        self.regs().sspcr1().modify(|_, w| w.sse().clear_bit());

        // SPI mode
        let (cpol, cpha) = match config.mode {
            SpiMode::Mode0 => (false, false),
            SpiMode::Mode1 => (false, true),
            SpiMode::Mode2 => (true, false),
            SpiMode::Mode3 => (true, true),
        };

        // Frame format
        self.regs().sspcr0().write(|w| unsafe {
            w.dss().bits((config.bits_per_word - 1) as u8);
            w.frf().bits(0); // Motorola SPI
            w.spo().bit(cpol);
            w.sph().bit(cpha);
            w.scr().bits(0);
            w
        });

        // Baudrate
        let mut cpsdvsr = 2u8;
        let mut scr = 0u8;

        let target = config.clk_peri / config.baudrate;

        'outer: for cps in (2u8..=254).step_by(2) {
            for s in 0u16..=255 {
                let div = (cps as u32) * ((s as u32) + 1);

                if div >= target {
                    cpsdvsr = cps;
                    scr = s as u8;
                    break 'outer;
                }
            }
        }

        self.regs()
            .sspcpsr()
            .write(|w| unsafe { w.cpsdvsr().bits(cpsdvsr) });

        self.regs()
            .sspcr0()
            .modify(|_, w| unsafe { w.scr().bits(scr) });

        // Master + enable
        self.regs().sspcr1().write(|w| {
            w.ms().clear_bit(); // master
            w.sse().set_bit() // enable
        });

        defmt::info!(
            "SPI CR0={:#x} CR1={:#x} CPSR={:#x} SR={:#x} cps={} scr={}",
            self.regs().sspcr0().read().bits(),
            self.regs().sspcr1().read().bits(),
            self.regs().sspcpsr().read().bits(),
            self.regs().sspsr().read().bits(),
            cpsdvsr,
            scr
        );
    }

    fn write(&self, data: &[u8]) {
        for &b in data {
            // TX FIFO not full
            while self.regs().sspsr().read().tnf().bit_is_clear() {}

            // Write data
            self.regs()
                .sspdr()
                .write(|w| unsafe { w.data().bits(b as u16) });

            // Wait RX
            while self.regs().sspsr().read().rne().bit_is_clear() {}

            // Drop RX
            let _ = self.regs().sspdr().read().data().bits();
        }

        // Wait until SPI idle
        while self.regs().sspsr().read().bsy().bit_is_set() {}
    }

    pub fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) {
        let len = core::cmp::min(tx.len(), rx.len());

        for i in 0..len {
            while self.regs().sspsr().read().tnf().bit_is_clear() {}

            self.regs()
                .sspdr()
                .write(|w| unsafe { w.data().bits(tx[i] as u16) });

            while self.regs().sspsr().read().rne().bit_is_clear() {}

            rx[i] = self.regs().sspdr().read().data().bits() as u8;
        }

        while self.regs().sspsr().read().bsy().bit_is_set() {}
    }

    fn read(&self, data: &mut [u8]) {
        for b in data.iter_mut() {
            while self.regs().sspsr().read().tnf().bit_is_clear() {}

            // Send dummy clock
            self.regs()
                .sspdr()
                .write(|w| unsafe { w.data().bits(0xFF) });

            while self.regs().sspsr().read().rne().bit_is_clear() {}

            *b = self.regs().sspdr().read().data().bits() as u8;
        }

        while self.regs().sspsr().read().bsy().bit_is_set() {}
    }
}

pub struct Pl022Spi {
    inner: IrqSafeNullLock<Pl022SpiInner>,
}

impl Pl022Spi {
    pub const COMPATIBLE: &'static str = "RP235x PL022 SPI";

    /// Create an instance
    pub const fn new(id: SpiId) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Pl022SpiInner::new(id)),
        }
    }
}

impl interface::SpiBus for Pl022Spi {
    fn config(&self, config: &SpiConfig) {
        self.inner.lock(|inner| inner.config(config));
    }
}

/// Device driver for PL011 UART
impl device_driver::interface::Driver for Pl022Spi {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| inner.init());
        Ok(())
    }
}

impl device_driver::interface::Device for Pl022Spi {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, data: &[u8]) -> Result<usize, device_driver::DevError> {
        self.inner.lock(|inner| inner.write(data));
        Ok(data.len())
    }
}

impl device_driver::interface::DeviceDriver for Pl022Spi {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}
