use defmt::info;
use rp235x_pac::interrupt;

use crate::sys::{
    interrupt::IrqHandlerDescriptor,
    synchronization::{IrqSafeNullLock, interface::Mutex},
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

    pub fn enable(&self, enable: bool) {
        if enable {
            unsafe {
                cortex_m::interrupt::enable();
            }
        } else {
            cortex_m::interrupt::disable();
        }
    }

    pub fn register_irq_handler(&self, descriptor: IrqHandlerDescriptor<rp235x_pac::Interrupt>) {
        let irq_number = descriptor.number();

        self.table.lock(|table| {
            table[irq_number as usize] = Some(descriptor);
        });

        cortex_m::peripheral::NVIC::unpend(irq_number);
        unsafe {
            cortex_m::peripheral::NVIC::unmask(irq_number);
        }
    }

    pub fn dispatch(&self, irq_number: rp235x_pac::Interrupt) -> Result<(), &'static str> {
        let handler = self
            .table
            .lock(|table| table[irq_number as usize].map(|descriptor| descriptor.handler()));

        if let Some(f) = handler {
            f.handler()?;
        }
        Ok(())
    }

    pub fn enumerate(&self) {
        for i in 0..INTR_CNT {
            if let Some(descriptor) = self.table.lock(|table| table[i]) {
                info!(
                    "     INT{} {}",
                    descriptor.number() as u16,
                    descriptor.name()
                );
            }
        }
    }
}

static IRQ_MANAGER: Rp235xIrqManger = Rp235xIrqManger::new();

pub fn irq_manager() -> &'static Rp235xIrqManger {
    &IRQ_MANAGER
}

#[interrupt]
fn UART0_IRQ() {
    info!("UART0 dispatch");

    if let Err(x) = IRQ_MANAGER.dispatch(rp235x_pac::Interrupt::UART0_IRQ) {
        panic!("UART0 IRQ failed: {}", x);
    }
}
