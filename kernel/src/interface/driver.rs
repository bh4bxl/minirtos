use crate::MemoryBlock;

pub struct DriverConfig<'a, I, D>
where
    I: Copy,
    D: Copy,
{
    pub dev_mem_blocks: &'a [MemoryBlock],
    pub interrupts: &'a [I],
    pub dmas: &'a [D],
}

pub trait Driver
where
    Self: Sized,
{
    type Interrupt: Copy;
    type Dma: Copy;
    type Error;

    fn new(config: DriverConfig<Self::Interrupt, Self::Dma>) -> Result<Self, Self::Error>;

    fn device_memory_blocks(&self) -> &[MemoryBlock];
}
