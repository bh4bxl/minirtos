use minirtos_abi::{EndpointHandle, MessageData, SysError, TaskId, UserMutPtr};

use crate::{
    ipc::{EndpointOwner, IPC_REGISTRY},
    synchronization::{CriticalSection, CriticalSectionLock},
};

mod memory;

pub(crate) use memory::MemoryOp;

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KernelServiceClass {
    Memory = 0,
    Dma = 1,
    Interrupt = 2,
}

impl TryFrom<u16> for KernelServiceClass {
    type Error = SysError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Memory),
            1 => Ok(Self::Dma),
            2 => Ok(Self::Interrupt),
            _ => Err(SysError::NotSupported),
        }
    }
}

#[inline]
pub(crate) const fn make_op(op: u16, sub_op: u16) -> u32 {
    ((op as u32) << 16) | sub_op as u32
}

#[inline]
pub(crate) const fn split_op(value: u32) -> (u16, u16) {
    ((value >> 16) as u16, value as u16)
}

pub(crate) struct KernelService {
    endpoint: Option<EndpointHandle>,
}

impl KernelService {
    pub(crate) const fn new() -> Self {
        Self { endpoint: None }
    }

    pub(crate) fn init(&mut self, cs: &CriticalSection) -> Result<(), SysError> {
        if self.endpoint.is_some() {
            return Ok(());
        }

        //
        // KernelService itself owns this endpoint.
        //
        let endpoint =
            IPC_REGISTRY.lock(cs, |registry| registry.create(EndpointOwner::KernelService))?;

        self.endpoint = Some(endpoint);

        Ok(())
    }

    pub(crate) fn endpoint(&self) -> Result<EndpointHandle, SysError> {
        self.endpoint.ok_or(SysError::InvalidState)
    }
}

static KERNEL_SERVICE: CriticalSectionLock<KernelService> =
    CriticalSectionLock::new(KernelService::new());

#[inline]
pub(crate) fn kernel_service() -> &'static CriticalSectionLock<KernelService> {
    &KERNEL_SERVICE
}

pub(crate) fn kernel_service_read(
    cs: &CriticalSection,
    sender: TaskId,
    op: u32,
    ptr: UserMutPtr<u8>,
    len: usize,
) -> Result<(), SysError> {
    let (class, sub_op) = split_op(op);

    match KernelServiceClass::try_from(class)? {
        KernelServiceClass::Memory => memory::handle_memory_read(cs, sender, sub_op, ptr, len),

        KernelServiceClass::Dma => Err(SysError::NotSupported),
        KernelServiceClass::Interrupt => Err(SysError::NotSupported),
    }
}

pub(crate) fn kernel_service_handle(
    cs: &CriticalSection,
    sender: TaskId,
    request: MessageData,
) -> Result<MessageData, SysError> {
    let (class, sub_op) = split_op(request.op);
    let class = KernelServiceClass::try_from(class)?;

    match class {
        KernelServiceClass::Memory => memory::handle_memory(cs, sender, sub_op, &request.args),

        KernelServiceClass::Dma => Err(SysError::NotSupported),

        KernelServiceClass::Interrupt => Err(SysError::NotSupported),
    }
}
