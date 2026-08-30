use core::arch;

use alloc::{vec, vec::Vec};

use minirtos_kernel::{MemoryBlock, kinfo};
use minirtos_services::driver::{
    DriverConfig, UartDriver,
    interface::Driver,
    uart::{DataBits, Parity, StopBits, UartConfig},
};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

use crate::DevError;

mod registers;

use registers::{CR, DR, FBRD, FR, IBRD, ICR, IMSC, LCR_H, Pl011Registers};

pub struct UartPl011<INTR, DMA> {
    dev_mem: [MemoryBlock; 1],
    interrupt: Option<INTR>,
    tx_dma: Option<DMA>,
    rx_dma: Option<DMA>,
}

impl<INTR, DMA> Driver for UartPl011<INTR, DMA> {
    type Interrupt = INTR;
    type Dma = DMA;
    type Error = DevError;

    fn new(config: DriverConfig<INTR, DMA>) -> Result<Self, Self::Error> {
        let Some(block) = config.dev_mem_blocks.first() else {
            return Err(DevError::InvalidArg);
        };

        let dev_mem = [MemoryBlock::new(block.base(), block.size())];

        let mut interrupts = config.interrupts.into_iter();
        let mut dmas = config.dmas.into_iter();

        Ok(Self {
            dev_mem,
            interrupt: interrupts.next(),
            tx_dma: dmas.next(),
            rx_dma: dmas.next(),
        })
    }

    fn device_memory(&self) -> &[MemoryBlock] {
        &self.dev_mem
    }
}

impl<INTR, DMA> UartPl011<INTR, DMA> {
    #[inline(always)]
    fn regs(&self) -> &Pl011Registers {
        unsafe { &*(self.dev_mem[0].base() as *const Pl011Registers) }
    }

    fn write_byte(&self, byte: u8) {
        while self.tx_fifo_full() {}

        self.regs().dr.write(DR::DATA.val(byte as u32));
    }

    fn read_byte(&self, blocking: bool) -> Option<u8> {
        while self.rx_fifo_empty() {
            if !blocking {
                return None;
            }
        }

        Some(self.regs().dr.read(DR::DATA) as u8)
    }

    fn enable(&self) {
        self.regs()
            .cr
            .modify(CR::UARTEN::SET + CR::TXE::SET + CR::RXE::SET);
    }

    fn disable(&self) {
        self.regs()
            .cr
            .modify(CR::UARTEN::CLEAR + CR::TXE::CLEAR + CR::RXE::CLEAR);
    }

    fn enable_irq(&self) {
        self.regs().imsc.modify(IMSC::RXIM::SET + IMSC::RTIM::SET);
    }

    fn disable_irq(&self) {
        self.regs()
            .imsc
            .modify(IMSC::RXIM::CLEAR + IMSC::RTIM::CLEAR);
    }

    fn clear_all_interrupts(&self) {
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

    fn set_baudrate(&self, uart_clock_hz: u32, baudrate: u32) -> Result<(), DevError> {
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

    fn configure_line_control(&self, config: &UartConfig) {
        let wlen = match config.data_bits {
            DataBits::Five => LCR_H::WLEN::FiveBits,
            DataBits::Six => LCR_H::WLEN::SixBits,
            DataBits::Seven => LCR_H::WLEN::SevenBits,
            DataBits::Eight => LCR_H::WLEN::EightBits,
        };

        let stop_bits = match config.stop_bits {
            StopBits::One => LCR_H::STP2::CLEAR,
            StopBits::Two => LCR_H::STP2::SET,
        };

        let parity = match config.parity {
            Parity::None => LCR_H::PEN::CLEAR + LCR_H::EPS::CLEAR,
            Parity::Even => LCR_H::PEN::SET + LCR_H::EPS::SET,
            Parity::Odd => LCR_H::PEN::SET + LCR_H::EPS::CLEAR,
        };

        self.regs()
            .lcr_h
            .write(LCR_H::FEN::SET + wlen + stop_bits + parity);
    }

    fn flush(&self) {
        while self.regs().fr.is_set(FR::BUSY) {}
    }

    fn tx_fifo_full(&self) -> bool {
        self.regs().fr.is_set(FR::TXFF)
    }

    fn rx_fifo_empty(&self) -> bool {
        self.regs().fr.is_set(FR::RXFE)
    }
}

impl<INTR, DMA> UartDriver for UartPl011<INTR, DMA> {
    type Error = DevError;

    fn init(&mut self) -> Result<(), Self::Error> {
        kinfo!(
            "pl011 init @ {:#010x} {}",
            &self.dev_mem[0].base(),
            &self.dev_mem[0].size()
        );
        Ok(())
    }

    fn config(&mut self, config: &UartConfig) -> Result<(), Self::Error> {
        kinfo!("pl011 config baud_rate {}", config.baud_rate);
        kinfo!("pl011 config data_bits {}", config.data_bits as u8);
        kinfo!("pl011 config stop_bits {}", config.stop_bits as u8);
        kinfo!("pl011 config parity {}", config.parity as u8);

        self.enable();

        self.clear_all_interrupts();

        self.set_baudrate(config.clock_hz, config.baud_rate)?;

        self.configure_line_control(&config);

        Ok(())
    }

    fn try_read_byte(&self) -> Result<Option<u8>, Self::Error> {
        kinfo!("pl011 try_read_byte");
        Ok(Some(60u8))
    }

    fn write_byte(&self, byte: u8) -> Result<(), Self::Error> {
        kinfo!("pl011 write_byte {}", byte as char);
        self.write_byte(byte);
        Ok(())
    }

    fn write_buf(&self, buf: &[u8]) -> Result<usize, Self::Error> {
        if let Ok(s) = core::str::from_utf8(buf) {
            kinfo!("pl011 write_buf: {}", s);
        }
        Ok(buf.len())
    }
}
