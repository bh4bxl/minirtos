use crate::{
    bsp::pac,
    drivers::spi::{SpiConfig, SpiMode, interface},
    sys::{
        device_driver::{self, DevError},
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
    dma: *const pac::dma::RegisterBlock,
    dma_tx_ch: usize,
    dma_rx_ch: usize,
    dma_tx_dreq: u8,
    dma_rx_dreq: u8,
}

const DREQ_SPI0_TX: u8 = 24;
const DREQ_SPI0_RX: u8 = 25;
const DREQ_SPI1_TX: u8 = 26;
const DREQ_SPI1_RX: u8 = 27;

#[allow(dead_code)]
impl Pl022SpiInner {
    /// Create an instance
    const fn new(id: SpiId) -> Self {
        let regs = match id {
            SpiId::SPI0 => unsafe { &*pac::SPI0::ptr() },
            SpiId::SPI1 => unsafe { &*pac::SPI1::ptr() },
        };
        let dma = unsafe { &*pac::DMA::ptr() };

        match id {
            SpiId::SPI0 => Self {
                id,
                regs,
                dma,
                dma_tx_ch: crate::drivers::dma::DmaChannles::Spi0Tx as usize,
                dma_rx_ch: crate::drivers::dma::DmaChannles::Spi0Rx as usize,
                dma_tx_dreq: DREQ_SPI0_TX,
                dma_rx_dreq: DREQ_SPI0_RX,
            },
            SpiId::SPI1 => Self {
                id,
                regs,
                dma,
                dma_tx_ch: crate::drivers::dma::DmaChannles::Spi1Tx as usize,
                dma_rx_ch: crate::drivers::dma::DmaChannles::Spi1Rx as usize,
                dma_tx_dreq: DREQ_SPI1_TX,
                dma_rx_dreq: DREQ_SPI1_RX,
            },
        }
    }

    fn id(&self) -> SpiId {
        self.id
    }

    fn regs(&self) -> &pac::spi0::RegisterBlock {
        unsafe { &*self.regs }
    }

    fn dma(&self) -> &pac::dma::RegisterBlock {
        unsafe { &*self.dma }
    }

    fn init(&self) {
        let resets = unsafe { &*pac::RESETS::ptr() };
        match self.id {
            SpiId::SPI0 => {
                resets.reset().modify(|_, w| w.spi0().set_bit());
                resets.reset().modify(|_, w| w.spi0().clear_bit());

                while resets.reset_done().read().spi0().bit_is_clear() {}
            }
            SpiId::SPI1 => {
                resets.reset().modify(|_, w| w.spi1().set_bit());
                resets.reset().modify(|_, w| w.spi1().clear_bit());

                while resets.reset_done().read().spi1().bit_is_clear() {}
            }
        }

        self.regs()
            .sspdmacr()
            .modify(|_, w| w.txdmae().set_bit().rxdmae().clear_bit());
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
    }

    fn dma_enable(&self, dir: super::DmaDir, enable: bool) {
        let (ch_id, dreq, incr_read, incr_write) = match dir {
            super::DmaDir::Tx => (
                self.dma_tx_ch,
                self.dma_tx_dreq,
                true,  // memory++
                false, // SPI DR fixed
            ),
            super::DmaDir::Rx => (
                self.dma_rx_ch,
                self.dma_rx_dreq,
                false, // SPI DR fixed
                true,  // memory++
            ),
        };

        let ch = self.dma().ch(ch_id);

        // disable first
        ch.ch_ctrl_trig().modify(|_, w| w.en().clear_bit());

        while ch.ch_ctrl_trig().read().busy().bit_is_set() {}

        if !enable {
            return;
        }

        // clear irq
        self.dma()
            .intr()
            .write(|w| unsafe { w.bits(1u32 << ch_id) });

        // configure
        ch.ch_ctrl_trig().write(|w| unsafe {
            w.data_size()
                .size_byte()
                .incr_read()
                .bit(incr_read)
                .incr_write()
                .bit(incr_write)
                .treq_sel()
                .bits(dreq)
                .en()
                .clear_bit()
        });
    }

    fn write(&self, data: &[u8]) -> Result<usize, crate::sys::device_driver::DevError> {
        let mut tx_idx = 0usize;
        let mut rx_left = data.len();

        while tx_idx < data.len() || rx_left > 0 {
            // Fill TX FIFO as much as possible
            while tx_idx < data.len() && self.regs().sspsr().read().tnf().bit_is_set() {
                self.regs()
                    .sspdr()
                    .write(|w| unsafe { w.data().bits(data[tx_idx] as u16) });

                tx_idx += 1;
            }

            // Drain RX FIFO as much as available
            while rx_left > 0 && self.regs().sspsr().read().rne().bit_is_set() {
                let _ = self.regs().sspdr().read().data().bits();
                rx_left -= 1;
            }
        }

        // Wait until SPI shift engine is idle
        while self.regs().sspsr().read().bsy().bit_is_set() {}

        Ok(data.len())
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

    fn spi_set_bits(&self, bits: u8) {
        debug_assert!((4..=16).contains(&bits));

        let regs = self.regs();
        while regs.sspsr().read().bsy().bit_is_set() {}

        regs.sspcr1().modify(|_, w| w.sse().clear_bit());
        regs.sspcr0()
            .modify(|_, w| unsafe { w.dss().bits(bits - 1) });
        regs.sspcr1().modify(|_, w| w.sse().set_bit());
    }

    fn dma_write_u8_blocking(
        &self,
        data: &[u8],
    ) -> Result<usize, crate::sys::device_driver::DevError> {
        if data.is_empty() {
            return Ok(0);
        }

        self.spi_set_bits(8);

        let ch = self.dma().ch(self.dma_tx_ch);

        // Stop the channel
        ch.ch_ctrl_trig().modify(|_, w| w.en().clear_bit());
        while ch.ch_ctrl_trig().read().busy().bit_is_set() {}

        // Clear interrupt
        self.dma()
            .intr()
            .write(|w| unsafe { w.bits(1 << self.dma_tx_ch) });

        // Set address
        ch.ch_read_addr()
            .write(|w| unsafe { w.bits(data.as_ptr() as u32) });
        let dr_addr = self.regs().sspdr().as_ptr() as u32;
        ch.ch_write_addr().write(|w| unsafe { w.bits(dr_addr) });

        // Set size
        ch.ch_trans_count()
            .write(|w| unsafe { w.bits(data.len() as u32) });

        // Enable channel
        ch.ch_ctrl_trig().modify(|_, w| unsafe {
            w.data_size()
                .size_byte()
                .incr_read()
                .set_bit()
                .incr_write()
                .clear_bit()
                .treq_sel()
                .bits(self.dma_tx_dreq)
                .en()
                .set_bit()
        });

        // Wait until done
        while ch.ch_ctrl_trig().read().busy().bit_is_set() {}

        // Wait until SPI shift engine is idle
        while self.regs().sspsr().read().bsy().bit_is_set() {}

        Ok(data.len())
    }

    fn dma_write_u16_blocking(&self, data: &[u16]) -> Result<usize, DevError> {
        if data.is_empty() {
            return Ok(0);
        }

        self.spi_set_bits(16);

        let ch = self.dma().ch(self.dma_tx_ch);

        ch.ch_ctrl_trig().modify(|_, w| w.en().clear_bit());
        while ch.ch_ctrl_trig().read().busy().bit_is_set() {}

        self.dma()
            .intr()
            .write(|w| unsafe { w.bits(1u32 << self.dma_tx_ch) });

        ch.ch_read_addr()
            .write(|w| unsafe { w.bits(data.as_ptr() as u32) });

        ch.ch_write_addr()
            .write(|w| unsafe { w.bits(self.regs().sspdr().as_ptr() as u32) });

        ch.ch_trans_count()
            .write(|w| unsafe { w.bits(data.len() as u32) });

        ch.ch_ctrl_trig().write(|w| unsafe {
            w.data_size()
                .size_halfword()
                .incr_read()
                .set_bit()
                .incr_write()
                .clear_bit()
                .treq_sel()
                .bits(self.dma_tx_dreq)
                .en()
                .set_bit()
        });

        while ch.ch_ctrl_trig().read().busy().bit_is_set() {}
        while self.regs().sspsr().read().bsy().bit_is_set() {}

        self.spi_set_bits(8);

        Ok(data.len() * 2)
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

    fn enable_dma(&self, dir: super::DmaDir, enable: bool) {
        self.inner.lock(|inner| inner.dma_enable(dir, enable));
    }

    fn write(&self, data: &[u8]) -> Result<usize, crate::sys::device_driver::DevError> {
        self.inner.lock(|inner| inner.write(data))
    }

    fn write_dma(&self, data: &[u8]) -> Result<usize, crate::sys::device_driver::DevError> {
        self.inner.lock(|inner| inner.dma_write_u8_blocking(data))
    }

    fn write_dma_u16(&self, data: &[u16]) -> Result<usize, crate::sys::device_driver::DevError> {
        self.inner.lock(|inner| inner.dma_write_u16_blocking(data))
    }
}

/// Device driver for PL011 UART
impl device_driver::interface::Driver for Pl022Spi {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init());
        Ok(())
    }
}

impl device_driver::interface::Device for Pl022Spi {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, data: &[u8]) -> Result<usize, device_driver::DevError> {
        self.inner.lock(|inner| inner.write(data))
    }
}

impl device_driver::interface::DeviceDriver for Pl022Spi {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}
