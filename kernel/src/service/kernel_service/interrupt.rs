use minirtos_abi::{InterruptHandle, MESSAGE_ARG_COUNT, MessageData, SysError, TaskId};

use crate::{
    interrupt::INTERRUPTS, ipc::EndpointOwner::KernelService, synchronization::CriticalSection,
};

use super::KernelServiceResult;

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InterruptOp {
    Claim = 0,
    Release = 1,
    Wait = 2,
}

impl TryFrom<u16> for InterruptOp {
    type Error = SysError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Claim),
            1 => Ok(Self::Release),
            2 => Ok(Self::Wait),
            _ => Err(SysError::NotSupported),
        }
    }
}

pub(super) fn handle_interrupt(
    cs: &CriticalSection,
    sender: TaskId,
    sub_op: u16,
    args: &[u32; MESSAGE_ARG_COUNT],
) -> Result<KernelServiceResult, SysError> {
    let op = InterruptOp::try_from(sub_op)?;

    match op {
        InterruptOp::Claim => {
            let irq = args[0];

            let handle = claim(cs, sender, irq)?;

            Ok(KernelServiceResult::Complete(MessageData::new(
                0,
                [handle.raw(), 0, 0, 0],
            )))
        }

        InterruptOp::Release => {
            let handle = InterruptHandle::from_raw(args[0]);

            release(cs, sender, handle)?;

            Ok(KernelServiceResult::Complete(MessageData::default()))
        }

        InterruptOp::Wait => {
            let handle = InterruptHandle::from_raw(args[0]);

            if wait(cs, sender, handle)? {
                Ok(KernelServiceResult::Complete(MessageData::default()))
            } else {
                Ok(KernelServiceResult::Blocked)
            }
        }
    }
}

fn claim(cs: &CriticalSection, owner: TaskId, irq: u32) -> Result<InterruptHandle, SysError> {
    INTERRUPTS.lock(cs, |registry| registry.claim(owner, irq))
}

fn release(cs: &CriticalSection, owner: TaskId, handle: InterruptHandle) -> Result<(), SysError> {
    INTERRUPTS.lock(cs, |registry| registry.release(owner, handle))
}

fn wait(cs: &CriticalSection, owner: TaskId, handle: InterruptHandle) -> Result<bool, SysError> {
    INTERRUPTS.lock(cs, |registry| registry.wait(owner, handle))
}
