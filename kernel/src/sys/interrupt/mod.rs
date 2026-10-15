use minirtos_abi::{InterruptHandle, MessageData, SysError};

use crate::service::{InterruptOp, KernelServiceClass, make_op};

use super::kernel_service_call;

pub struct Interrupt {
    handle: InterruptHandle,
}

impl Interrupt {
    pub fn claim(irq: u32) -> Result<Self, SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Interrupt as u16,
                InterruptOp::Claim as u16,
            ),
            [irq, 0, 0, 0],
        );

        let response = kernel_service_call(&request)?;

        Ok(Self {
            handle: InterruptHandle::from_raw(response.args[0]),
        })
    }

    pub fn wait(&self) -> Result<(), SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Interrupt as u16,
                InterruptOp::Wait as u16,
            ),
            [self.handle.raw(), 0, 0, 0],
        );

        kernel_service_call(&request)?;

        Ok(())
    }

    pub fn release(self) -> Result<(), SysError> {
        let request = MessageData::new(
            make_op(
                KernelServiceClass::Interrupt as u16,
                InterruptOp::Release as u16,
            ),
            [self.handle.raw(), 0, 0, 0],
        );

        kernel_service_call(&request)?;

        Ok(())
    }

    pub fn handle(&self) -> InterruptHandle {
        self.handle
    }
}
