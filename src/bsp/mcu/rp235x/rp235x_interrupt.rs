use rp235x_pac::interrupt;

use crate::{
    m_info,
    sys::{
        interrupt::{IrqHandlerDescriptor, irq_manager},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

// A simple PICO implementaiton

const INTR_CNT: usize = 52;

pub struct Rp235xIrqManger {
    table: IrqSafeNullLock<[Option<IrqHandlerDescriptor<rp235x_pac::Interrupt>>; INTR_CNT]>,
}

impl Rp235xIrqManger {
    pub const fn new() -> Self {
        Self {
            table: IrqSafeNullLock::new([None; INTR_CNT]),
        }
    }
}

impl crate::sys::interrupt::interface::IrqManager for Rp235xIrqManger {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn register_irq_handler(
        &self,
        descriptor: IrqHandlerDescriptor<Self::IrqNumberType>,
    ) -> Result<(), &'static str> {
        let irq_number = descriptor.number();

        self.table.lock(|table| {
            table[irq_number as usize] = Some(descriptor);
        });

        cortex_m::peripheral::NVIC::unpend(irq_number);
        unsafe {
            cortex_m::peripheral::NVIC::unmask(irq_number);
        }

        Ok(())
    }

    fn enable(&self, enable: bool) {
        if enable {
            unsafe {
                cortex_m::interrupt::enable();
            }
        } else {
            cortex_m::interrupt::disable();
        }
    }

    fn dispatch(&self, irq_number: Self::IrqNumberType) -> Result<(), &'static str> {
        let handler = self
            .table
            .lock(|table| table[irq_number as usize].map(|descriptor| descriptor.handler()));

        if let Some(f) = handler {
            f.handler()?;
        }
        Ok(())
    }

    fn enumerate(&self) {
        for i in 0..INTR_CNT {
            if let Some(descriptor) = self.table.lock(|table| table[i]) {
                m_info!(
                    "     INT{} {}",
                    descriptor.number() as u16,
                    descriptor.name()
                );
            }
        }
    }
}

#[interrupt]
fn UART0_IRQ() {
    if let Err(x) = irq_manager().dispatch(rp235x_pac::Interrupt::UART0_IRQ) {
        panic!("UART0 IRQ failed: {}", x);
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    if let Err(x) = irq_manager().dispatch(rp235x_pac::Interrupt::IO_IRQ_BANK0) {
        panic!("IO_IRQ_BANK0 failed: {}", x);
    }
}
