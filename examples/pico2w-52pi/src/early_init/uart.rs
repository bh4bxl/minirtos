use minirtos_drivers::{DevError, uart::Pl011};
use minirtos_kernel::{
    MemoryBlock,
    interface::driver::{Driver, DriverConfig},
    stdio::register_console,
};
use minirtos_services::driver::{UartDriver, uart::UartConfig};

pub(super) struct Uart {
    inner: Pl011<(), ()>,
}

impl Uart {
    pub(super) fn new(base: usize) -> Result<Self, DevError> {
        let dev_config = DriverConfig::<(), ()> {
            dev_mem_blocks: &[MemoryBlock::new(base, 0x1000)],
            interrupts: &[],
            dmas: &[],
        };

        Ok(Self {
            inner: Pl011::new(dev_config)?,
        })
    }

    pub(super) fn init(&'static self, sys_clk_hz: u32) -> Result<(), DevError> {
        let mut config = UartConfig::default();
        config.clock_hz = sys_clk_hz;

        self.inner.init()?;
        self.inner.config(&config)?;

        register_console(&self.inner);

        Ok(())
    }
}
