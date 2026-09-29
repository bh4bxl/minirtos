use minirtos_kernel::{MemoryBlock, interface::driver::DriverConfig};
use minirtos_services::driver::uart::{DataBits, Parity, StopBits};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::DevError;

use super::registers::*;

pub struct Pl011Inner<I, D> {
    dev_mem: [MemoryBlock; 1],
    tx_interrupt: Option<I>,
    rx_interrupt: Option<I>,
    tx_dma: Option<D>,
    rx_dma: Option<D>,
}

impl<I, D> Pl011Inner<I, D>
where
    I: Copy,
    D: Copy,
{
    pub(super) fn new(config: DriverConfig<I, D>) -> Result<Self, DevError> {
        let Some(block) = config.dev_mem_blocks.first() else {
            return Err(DevError::InvalidArg);
        };

        let dev_mem = [*block];

        Ok(Self {
            dev_mem,
            tx_interrupt: config.interrupts.first().copied(),
            rx_interrupt: config.interrupts.get(1).copied(),
            tx_dma: config.dmas.first().copied(),
            rx_dma: config.dmas.get(1).copied(),
        })
    }

    pub(super) fn device_memory_blocks(&self) -> &[MemoryBlock] {
        &self.dev_mem
    }

    #[inline(always)]
    fn regs(&self) -> &Pl011Registers {
        unsafe { &*(self.dev_mem[0].base() as *const Pl011Registers) }
    }

    pub(super) fn enable(&self) {
        self.regs()
            .cr
            .modify(CR::UARTEN::SET + CR::TXE::SET + CR::RXE::SET);
    }

    pub(super) fn disable(&self) {
        self.regs()
            .cr
            .modify(CR::UARTEN::CLEAR + CR::TXE::CLEAR + CR::RXE::CLEAR);
    }

    pub(super) fn clear_all_interrupts(&self) {
        self.regs().icr.write(
            ICR::RIMIC::SET
                + ICR::CTSMIC::SET
                + ICR::DCDMIC::SET
                + ICR::DSRMIC::SET
                + ICR::RXIC::SET
                + ICR::TXIC::SET
                + ICR::RTIC::SET
                + ICR::FEIC::SET
                + ICR::PEIC::SET
                + ICR::BEIC::SET
                + ICR::OEIC::SET,
        );
    }

    pub(super) fn set_baudrate(&self, uart_clock_hz: u32, baudrate: u32) -> Result<(), DevError> {
        if baudrate == 0 {
            return Err(DevError::InvalidArg);
        }

        let baud_x64 = ((4 * uart_clock_hz) + (baudrate / 2)) / baudrate;
        let ibrd = baud_x64 / 64;
        let fbrd = baud_x64 % 64;

        self.regs().ibrd.write(IBRD::BAUD_DIVINT.val(ibrd));
        self.regs().fbrd.write(FBRD::BAUD_DIVFRAC.val(fbrd));

        Ok(())
    }

    pub(super) fn configure_line_control(&self, db: DataBits, sb: StopBits, p: Parity) {
        let wlen = match db {
            DataBits::Five => LCR_H::WLEN::FiveBits,
            DataBits::Six => LCR_H::WLEN::SixBits,
            DataBits::Seven => LCR_H::WLEN::SevenBits,
            DataBits::Eight => LCR_H::WLEN::EightBits,
        };

        let stop_bits = match sb {
            StopBits::One => LCR_H::STP2::CLEAR,
            StopBits::Two => LCR_H::STP2::SET,
        };

        let parity = match p {
            Parity::None => LCR_H::PEN::CLEAR + LCR_H::EPS::CLEAR,
            Parity::Even => LCR_H::PEN::SET + LCR_H::EPS::SET,
            Parity::Odd => LCR_H::PEN::SET + LCR_H::EPS::CLEAR,
        };

        self.regs()
            .lcr_h
            .write(LCR_H::FEN::SET + wlen + stop_bits + parity);
    }

    pub(super) fn tx_fifo_full(&self) -> bool {
        self.regs().fr.is_set(FR::TXFF)
    }

    pub(super) fn rx_fifo_empty(&self) -> bool {
        self.regs().fr.is_set(FR::RXFE)
    }

    pub(super) fn write_byte(&self, byte: u8) {
        while self.tx_fifo_full() {}

        self.regs().dr.write(DR::DATA.val(byte as u32));
    }

    pub(super) fn read_byte(&self, blocking_mode: super::BlockingMode) -> Option<u8> {
        while self.rx_fifo_empty() {
            if matches!(blocking_mode, super::BlockingMode::NonBlocking) {
                return None;
            }
        }

        Some(self.regs().dr.read(DR::DATA) as u8)
    }

    pub(super) fn flush(&self) {
        while self.regs().fr.is_set(FR::TXFF) {}
    }
}

impl<I, D> core::fmt::Write for Pl011Inner<I, D>
where
    I: Copy,
    D: Copy,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_byte(c as u8);
        }

        Ok(())
    }
}
