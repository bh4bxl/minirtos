use cortex_m::{interrupt::InterruptNumber, peripheral::NVIC};
use rp235x_pac::{Interrupt, interrupt};

#[interrupt]
fn UART0_IRQ() {
    NVIC::mask(Interrupt::UART0_IRQ);

    minirtos_kernel::interrupt::handle_irq(Interrupt::UART0_IRQ.number() as u32);
}
