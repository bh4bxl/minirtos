use cortex_m::peripheral::SYST;
use defmt::info;
use rp235x_pac::interrupt;

use crate::sys::{
    device_driver::{self, DeviceType},
    interrupt::{IrqHandlerDescriptor, irq_manager},
    scheduler,
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
                info!(
                    "     INT{} {}",
                    descriptor.number() as u16,
                    descriptor.name()
                );
            }
        }
    }
}

pub fn systick_init(mut syst: SYST, cpu_hz: u32, tick_hz: u32) {
    let reload = cpu_hz / tick_hz - 1;

    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(reload);
    syst.clear_current();

    syst.enable_interrupt();
    syst.enable_counter();
}

#[interrupt]
fn UART0_IRQ() {
    info!("UART0 dispatch");

    if let Err(x) = irq_manager().dispatch(rp235x_pac::Interrupt::UART0_IRQ) {
        panic!("UART0 IRQ failed: {}", x);
    }
}

#[cortex_m_rt::exception]
fn SysTick() {
    if let Some(gpio) = device_driver::driver_manager().open_device(DeviceType::Gpio, 0) {
        let mut data = [19u8, 0];
        if let Err(_x) = gpio.read(&mut data) {
            defmt::error!("GPIO read failed: {}", data[0]);
            return;
        }

        data[1] = if data[1] == 0 { 1 } else { 0 };

        if let Err(_x) = gpio.write(&data) {
            defmt::error!("GPIO write failed: {}", data[0]);
        }
    }
}

#[cortex_m_rt::exception]
unsafe fn SVCall() {
    unsafe {
        let sp = scheduler::scheduler().current_task_sp();
        core::arch::asm!(
            // Restore r4-r11 from task stack
            "ldmia {sp}!, {{r4-r11}}",
            // PSP = remaining hardware frame
            "msr psp, {sp}",
            // Thread mode use PSP
            "movs r0, #2",
            "msr CONTROL, r0",
            "isb",
            // Exception return to thread mode using PSP
            "ldr lr, =0xFFFFFFFD",
            "bx lr",
            sp = in(reg) sp,
            options(noreturn)
        );
    }
}
