mod serivce_table;
pub mod uart;

use alloc::vec::Vec;
use minirtos_kernel::MemoryBlock;
pub use serivce_table::{DriverService, DriverServiceConfig, DriverServiceTable};
pub use uart::{
    interface::UartDriver,
    uart_client::{Uart, UartId},
    uart_service::UartService,
};

pub struct DriverConfig<I, D> {
    pub dev_mem_blocks: Vec<MemoryBlock>,
    pub interrupts: Vec<I>,
    pub dmas: Vec<D>,
}

pub mod interface {
    use minirtos_kernel::MemoryBlock;

    pub trait Driver
    where
        Self: Sized,
    {
        type Interrupt;
        type Dma;
        type Error;

        fn new(
            config: super::DriverConfig<Self::Interrupt, Self::Dma>,
        ) -> Result<Self, Self::Error>;

        fn device_memory(&self) -> &[MemoryBlock];
    }
}
