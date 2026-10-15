use minirtos_abi::{InterruptHandle, MessageData, TaskId};

use crate::{
    arch,
    service::kernel_service,
    synchronization::{CriticalSectionLock, critical_section},
    sys::complete_pending_call,
};

mod registry;

use registry::InterruptRegistry;

struct Interrupt {
    irq: u32,
    owner: TaskId,
    pending: bool,
    waiter: Option<TaskId>,
}

pub(crate) static INTERRUPTS: CriticalSectionLock<InterruptRegistry> =
    CriticalSectionLock::new(InterruptRegistry::new());

pub fn handle_irq(irq: u32) {
    critical_section(|cs| {
        let waiter = INTERRUPTS.lock(cs, |registry| registry.handle_irq(irq));

        if let Some(task) = waiter {
            let result = (|| {
                let endpoint = kernel_service().lock(cs, |service| service.endpoint())?;

                complete_pending_call(cs, task, endpoint, MessageData::default())
            })();

            if let Err(err) = result {
                defmt::error!("IRQ {} completion failed: {:?}", irq, err as i32);
            } else {
                arch::request_context_switch();
            }
        }
    });
}
