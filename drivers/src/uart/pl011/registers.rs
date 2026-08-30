use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

register_bitfields![u32,
    /// Data Register
    pub(super) DR [
        DATA   OFFSET(0) NUMBITS(8) [],
        FE     OFFSET(8) NUMBITS(1) [],
        PE     OFFSET(9) NUMBITS(1) [],
        BE     OFFSET(10) NUMBITS(1) [],
        OE     OFFSET(11) NUMBITS(1) []
    ],

    /// Receive Status / Error Clear Register
    pub(super) RSR_ECR [
        FE     OFFSET(0) NUMBITS(1) [],
        PE     OFFSET(1) NUMBITS(1) [],
        BE     OFFSET(2) NUMBITS(1) [],
        OE     OFFSET(3) NUMBITS(1) []
    ],

    /// Flag Register
    pub(super) FR [
        CTS    OFFSET(0) NUMBITS(1) [],
        DSR    OFFSET(1) NUMBITS(1) [],
        DCD    OFFSET(2) NUMBITS(1) [],
        BUSY   OFFSET(3) NUMBITS(1) [],
        RXFE   OFFSET(4) NUMBITS(1) [],
        TXFF   OFFSET(5) NUMBITS(1) [],
        RXFF   OFFSET(6) NUMBITS(1) [],
        TXFE   OFFSET(7) NUMBITS(1) [],
        RI     OFFSET(8) NUMBITS(1) []
    ],

    /// IrDA Low-Power Counter Register
    pub(super) ILPR [
        ILPDVSR OFFSET(0) NUMBITS(8) []
    ],

    /// Integer Baud Rate Register
    pub(super) IBRD [
        BAUD_DIVINT OFFSET(0) NUMBITS(16) []
    ],

    /// Fractional Baud Rate Register
    pub(super) FBRD [
        BAUD_DIVFRAC OFFSET(0) NUMBITS(6) []
    ],

    /// Line Control Register
    pub(super) LCR_H [
        BRK    OFFSET(0) NUMBITS(1) [],
        PEN    OFFSET(1) NUMBITS(1) [],
        EPS    OFFSET(2) NUMBITS(1) [],
        STP2   OFFSET(3) NUMBITS(1) [],
        FEN    OFFSET(4) NUMBITS(1) [],

        WLEN   OFFSET(5) NUMBITS(2) [
            FiveBits  = 0b00,
            SixBits   = 0b01,
            SevenBits = 0b10,
            EightBits = 0b11
        ],

        SPS    OFFSET(7) NUMBITS(1) []
    ],

    /// Control Register
    pub(super) CR [
        UARTEN OFFSET(0) NUMBITS(1) [],
        SIREN  OFFSET(1) NUMBITS(1) [],
        SIRLP  OFFSET(2) NUMBITS(1) [],

        LBE    OFFSET(7) NUMBITS(1) [],
        TXE    OFFSET(8) NUMBITS(1) [],
        RXE    OFFSET(9) NUMBITS(1) [],
        DTR    OFFSET(10) NUMBITS(1) [],
        RTS    OFFSET(11) NUMBITS(1) [],
        OUT1   OFFSET(12) NUMBITS(1) [],
        OUT2   OFFSET(13) NUMBITS(1) [],
        RTSEN  OFFSET(14) NUMBITS(1) [],
        CTSEN  OFFSET(15) NUMBITS(1) []
    ],

    /// Interrupt FIFO Level Select Register
    pub(super) IFLS [
        TXIFLSEL OFFSET(0) NUMBITS(3) [
            OneEighth     = 0b000,
            OneQuarter    = 0b001,
            OneHalf       = 0b010,
            ThreeQuarters = 0b011,
            SevenEighths  = 0b100
        ],

        RXIFLSEL OFFSET(3) NUMBITS(3) [
            OneEighth     = 0b000,
            OneQuarter    = 0b001,
            OneHalf       = 0b010,
            ThreeQuarters = 0b011,
            SevenEighths  = 0b100
        ]
    ],

    /// Interrupt Mask Set/Clear Register
    pub(super) IMSC [
        RIMIM  OFFSET(0) NUMBITS(1) [],
        CTSMIM OFFSET(1) NUMBITS(1) [],
        DCDMIM OFFSET(2) NUMBITS(1) [],
        DSRMIM OFFSET(3) NUMBITS(1) [],
        RXIM   OFFSET(4) NUMBITS(1) [],
        TXIM   OFFSET(5) NUMBITS(1) [],
        RTIM   OFFSET(6) NUMBITS(1) [],
        FEIM   OFFSET(7) NUMBITS(1) [],
        PEIM   OFFSET(8) NUMBITS(1) [],
        BEIM   OFFSET(9) NUMBITS(1) [],
        OEIM   OFFSET(10) NUMBITS(1) []
    ],

    /// Raw Interrupt Status Register
    pub(super) RIS [
        RIRMIS  OFFSET(0) NUMBITS(1) [],
        CTSRMIS OFFSET(1) NUMBITS(1) [],
        DCDRMIS OFFSET(2) NUMBITS(1) [],
        DSRRMIS OFFSET(3) NUMBITS(1) [],
        RXRIS   OFFSET(4) NUMBITS(1) [],
        TXRIS   OFFSET(5) NUMBITS(1) [],
        RTRIS   OFFSET(6) NUMBITS(1) [],
        FERIS   OFFSET(7) NUMBITS(1) [],
        PERIS   OFFSET(8) NUMBITS(1) [],
        BERIS   OFFSET(9) NUMBITS(1) [],
        OERIS   OFFSET(10) NUMBITS(1) []
    ],

    /// Masked Interrupt Status Register
    pub(super) MIS [
        RIMMIS  OFFSET(0) NUMBITS(1) [],
        CTSMMIS OFFSET(1) NUMBITS(1) [],
        DCDMMIS OFFSET(2) NUMBITS(1) [],
        DSRMMIS OFFSET(3) NUMBITS(1) [],
        RXMIS   OFFSET(4) NUMBITS(1) [],
        TXMIS   OFFSET(5) NUMBITS(1) [],
        RTMIS   OFFSET(6) NUMBITS(1) [],
        FEMIS   OFFSET(7) NUMBITS(1) [],
        PEMIS   OFFSET(8) NUMBITS(1) [],
        BEMIS   OFFSET(9) NUMBITS(1) [],
        OEMIS   OFFSET(10) NUMBITS(1) []
    ],

    /// Interrupt Clear Register
    pub(super) ICR [
        RIMIC   OFFSET(0) NUMBITS(1) [],
        CTSMIC  OFFSET(1) NUMBITS(1) [],
        DCDMIC  OFFSET(2) NUMBITS(1) [],
        DSRMIC  OFFSET(3) NUMBITS(1) [],
        RXIC    OFFSET(4) NUMBITS(1) [],
        TXIC    OFFSET(5) NUMBITS(1) [],
        RTIC    OFFSET(6) NUMBITS(1) [],
        FEIC    OFFSET(7) NUMBITS(1) [],
        PEIC    OFFSET(8) NUMBITS(1) [],
        BEIC    OFFSET(9) NUMBITS(1) [],
        OEIC    OFFSET(10) NUMBITS(1) []
    ],

    /// DMA Control Register
    pub(super) DMACR [
        RXDMAE    OFFSET(0) NUMBITS(1) [],
        TXDMAE    OFFSET(1) NUMBITS(1) [],
        DMAONERR  OFFSET(2) NUMBITS(1) []
    ]
];

register_structs! {
    pub Pl011Registers {
        /// Data Register
        (0x000 => pub dr: ReadWrite<u32, DR::Register>),

        /// Receive Status / Error Clear Register
        (0x004 => pub rsr_ecr: ReadWrite<u32, RSR_ECR::Register>),

        (0x008 => _reserved0),

        /// Flag Register
        (0x018 => pub fr: ReadOnly<u32, FR::Register>),

        (0x01C => _reserved1),

        /// IrDA Low-Power Counter Register
        (0x020 => pub ilpr: ReadWrite<u32, ILPR::Register>),

        /// Integer Baud Rate Register
        (0x024 => pub ibrd: ReadWrite<u32, IBRD::Register>),

        /// Fractional Baud Rate Register
        (0x028 => pub fbrd: ReadWrite<u32, FBRD::Register>),

        /// Line Control Register
        (0x02C => pub lcr_h: ReadWrite<u32, LCR_H::Register>),

        /// Control Register
        (0x030 => pub cr: ReadWrite<u32, CR::Register>),

        /// Interrupt FIFO Level Select Register
        (0x034 => pub ifls: ReadWrite<u32, IFLS::Register>),

        /// Interrupt Mask Set/Clear Register
        (0x038 => pub imsc: ReadWrite<u32, IMSC::Register>),

        /// Raw Interrupt Status Register
        (0x03C => pub ris: ReadOnly<u32, RIS::Register>),

        /// Masked Interrupt Status Register
        (0x040 => pub mis: ReadOnly<u32, MIS::Register>),

        /// Interrupt Clear Register
        (0x044 => pub icr: WriteOnly<u32, ICR::Register>),

        /// DMA Control Register
        (0x048 => pub dmacr: ReadWrite<u32, DMACR::Register>),

        (0x04C => _reserved2),

        // PL011 PrimeCell identification registers.
        (0xFE0 => pub periph_id0: ReadOnly<u32>),
        (0xFE4 => pub periph_id1: ReadOnly<u32>),
        (0xFE8 => pub periph_id2: ReadOnly<u32>),
        (0xFEC => pub periph_id3: ReadOnly<u32>),

        (0xFF0 => pub pcell_id0: ReadOnly<u32>),
        (0xFF4 => pub pcell_id1: ReadOnly<u32>),
        (0xFF8 => pub pcell_id2: ReadOnly<u32>),
        (0xFFC => pub pcell_id3: ReadOnly<u32>),

        (0x1000 => @END),
    }
}
