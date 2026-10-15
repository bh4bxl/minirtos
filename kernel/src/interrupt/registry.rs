use minirtos_abi::{InterruptHandle, SysError, TaskId};

use crate::arch;

use super::Interrupt;

const MAX_INTERRUPTS: usize = 16;

pub(crate) struct InterruptRegistry {
    interrupts: [Option<Interrupt>; MAX_INTERRUPTS],
}

impl InterruptRegistry {
    pub(super) const fn new() -> Self {
        Self {
            interrupts: [const { None }; MAX_INTERRUPTS],
        }
    }

    pub(crate) fn claim(&mut self, owner: TaskId, irq: u32) -> Result<InterruptHandle, SysError> {
        if irq as usize >= arch::max_irqs() {
            return Err(SysError::InvalidArgument);
        }

        // IRQ can only be claimed once.
        if self
            .interrupts
            .iter()
            .flatten()
            .any(|interrupt| interrupt.irq == irq)
        {
            return Err(SysError::Busy);
        }

        let index = self
            .interrupts
            .iter()
            .position(Option::is_none)
            .ok_or(SysError::NoResource)?;

        self.interrupts[index] = Some(Interrupt {
            irq,
            owner,
            pending: false,
            waiter: None,
        });

        arch::clear_pending_irq(irq);
        arch::enable_irq(irq);

        Ok(InterruptHandle::from_raw(index as u32))
    }

    pub(crate) fn release(
        &mut self,
        owner: TaskId,
        handle: InterruptHandle,
    ) -> Result<(), SysError> {
        let index = handle.raw() as usize;

        let interrupt = self
            .interrupts
            .get(index)
            .and_then(Option::as_ref)
            .ok_or(SysError::NotFound)?;

        if interrupt.owner != owner {
            return Err(SysError::PermissionDenied);
        }

        if interrupt.waiter.is_some() {
            return Err(SysError::Busy);
        }

        let irq = interrupt.irq;

        arch::disable_irq(irq);
        arch::clear_pending_irq(irq);

        self.interrupts[index] = None;

        Ok(())
    }

    pub(crate) fn wait(
        &mut self,
        owner: TaskId,
        handle: InterruptHandle,
    ) -> Result<bool, SysError> {
        let interrupt = self
            .interrupts
            .get_mut(handle.raw() as usize)
            .and_then(Option::as_mut)
            .ok_or(SysError::NotFound)?;

        if interrupt.owner != owner {
            return Err(SysError::PermissionDenied);
        }

        if interrupt.pending {
            interrupt.pending = false;
            arch::enable_irq(interrupt.irq);
            return Ok(true);
        }

        if interrupt.waiter.is_some() {
            return Err(SysError::Busy);
        }

        interrupt.waiter = Some(owner);

        arch::enable_irq(interrupt.irq);

        Ok(false)
    }

    pub(crate) fn handle_irq(&mut self, irq: u32) -> Option<TaskId> {
        let Some(interrupt) = self
            .interrupts
            .iter_mut()
            .flatten()
            .find(|interrupt| interrupt.irq == irq)
        else {
            return None;
        };

        if let Some(waiter) = interrupt.waiter.take() {
            Some(waiter)
        } else {
            interrupt.pending = true;
            None
        }
    }
}
