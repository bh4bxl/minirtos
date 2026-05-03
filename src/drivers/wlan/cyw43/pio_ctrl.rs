use pio::{
    Instruction, InstructionOperands, JmpCondition, MovDestination, MovOperation, MovSource,
    OutDestination,
};
use rp235x_hal::{
    pac,
    pio::{InstalledProgram, SM0, StateMachine, Stopped},
};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PioId {
    Pio0,
    Pio1,
    Pio2,
}

pub(super) struct PioCtrl {
    pio: *const pac::pio0::RegisterBlock,
    sm_index: u8,
    sm_mask: u8,

    program_offset: u8,
}

const SPI_OFFSET_LP: u8 = 0;
const SPI_OFFSET_LP1_END: u8 = 2;
const SPI_OFFSET_END: u8 = 6;

#[allow(dead_code)]
impl PioCtrl {
    pub const fn new(id: PioId) -> Self {
        let pio = unsafe {
            match id {
                PioId::Pio0 => &*pac::PIO0::ptr(),
                PioId::Pio1 => &*pac::PIO1::ptr(),
                PioId::Pio2 => &*pac::PIO2::ptr(),
            }
        };
        Self {
            sm_index: 1,
            sm_mask: 0,
            pio,
            program_offset: 0,
        }
    }

    #[inline]
    fn pio(&self) -> &pac::pio0::RegisterBlock {
        unsafe { &*self.pio }
    }

    pub fn load_program(
        &mut self,
        sm_index: u8,
        installed: InstalledProgram<pac::PIO0>,
    ) -> InstalledProgram<pac::PIO0> {
        self.sm_index = sm_index;
        self.sm_mask = 1 << sm_index;

        self.program_offset = installed.offset();

        installed
    }

    #[inline]
    pub fn exec_out_x_32(&mut self, sm: &mut StateMachine<(pac::PIO0, SM0), Stopped>) {
        sm.exec_instruction(Instruction {
            operands: InstructionOperands::OUT {
                destination: OutDestination::X,
                bit_count: 32,
            },
            delay: 0,
            side_set: Some(0),
        });
    }

    #[inline]
    pub fn exec_out_y_32(&mut self, sm: &mut StateMachine<(pac::PIO0, SM0), Stopped>) {
        sm.exec_instruction(Instruction {
            operands: InstructionOperands::OUT {
                destination: OutDestination::Y,
                bit_count: 32,
            },
            delay: 0,
            side_set: Some(0),
        });
    }

    #[inline]
    pub fn wait_idle(&self) {
        self.pio()
            .fdebug()
            .write(|w| unsafe { w.txstall().bits(self.sm_mask) });

        while self.pio().fdebug().read().txstall().bits() & self.sm_mask == 0 {
            cortex_m::asm::nop();
        }
    }

    #[inline]
    fn set_wrap(&self, target: u8, source: u8) {
        let wrap_bottom = self.program_offset + target;
        let wrap_top = self.program_offset + source;

        let pio = unsafe { &*self.pio };

        pio.sm(self.sm_index as usize)
            .sm_execctrl()
            .modify(|_, w| unsafe {
                w.wrap_bottom().bits(wrap_bottom);
                w.wrap_top().bits(wrap_top)
            });
    }

    #[inline]
    pub fn set_wrap_tx_only(&self) {
        self.set_wrap(SPI_OFFSET_LP, SPI_OFFSET_LP1_END - 1);
    }

    #[inline]
    pub fn set_wrap_tx_rx(&self) {
        self.set_wrap(SPI_OFFSET_LP, SPI_OFFSET_END - 1);
    }

    #[inline]
    pub fn exec_jmp_start(&self, sm: &mut StateMachine<(pac::PIO0, SM0), Stopped>) {
        sm.exec_instruction(Instruction {
            operands: InstructionOperands::JMP {
                condition: JmpCondition::Always,
                address: self.program_offset,
            },
            delay: 0,
            side_set: Some(0),
        });
    }

    #[inline]
    pub fn exec_mov_pins_null(&mut self, sm: &mut StateMachine<(pac::PIO0, SM0), Stopped>) {
        sm.exec_instruction(Instruction {
            operands: InstructionOperands::MOV {
                destination: MovDestination::PINS,
                source: MovSource::NULL,
                op: MovOperation::None,
            },
            delay: 0,
            side_set: Some(0),
        });
    }
}
