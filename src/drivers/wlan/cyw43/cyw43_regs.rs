#![allow(dead_code)]

pub(crate) const SDIO_FUNCTION2_WATERMARK: u32 = 0x10008;
pub(crate) const SDIO_BACKPLANE_ADDRESS_LOW: u32 = 0x1000a;
pub(crate) const SDIO_BACKPLANE_ADDRESS_MID: u32 = 0x1000b;
pub(crate) const SDIO_BACKPLANE_ADDRESS_HIGH: u32 = 0x1000c;
pub(crate) const SDIO_CHIP_CLOCK_CSR: u32 = 0x1000e;
pub(crate) const SDIO_WAKEUP_CTRL: u32 = 0x1001e;
pub(crate) const SDIO_SLEEP_CSR: u32 = 0x1001f;

pub(crate) const I_HMB_SW_MASK: u32 = 0x0000_00f0;
pub(crate) const I_HMB_FC_CHANGE: u32 = 1 << 5;

pub(crate) const CHIPCOMMON_BASE_ADDRESS: u32 = 0x1800_0000;
pub(crate) const SDIO_BASE_ADDRESS: u32 = 0x1800_2000;
pub(crate) const WLAN_ARMCM3_BASE_ADDRESS: u32 = 0x1800_3000;
pub(crate) const SOCSRAM_BASE_ADDRESS: u32 = 0x1800_4000;
pub(crate) const BACKPLANE_ADDR_MASK: u32 = 0x7fff;
pub(crate) const WRAPPER_REGISTER_OFFSET: u32 = 0x10_0000;

pub(crate) const SBSDIO_SB_ACCESS_2_4B_FLAG: u32 = 0x08000;

pub(crate) const CHIPCOMMON_SR_CONTROL1: u32 = CHIPCOMMON_BASE_ADDRESS + 0x508;
pub(crate) const SDIO_INT_STATUS: u32 = SDIO_BASE_ADDRESS + 0x20;
pub(crate) const SDIO_INT_HOST_MASK: u32 = SDIO_BASE_ADDRESS + 0x24;
pub(crate) const SDIO_FUNCTION_INT_MASK: u32 = SDIO_BASE_ADDRESS + 0x34;
pub(crate) const SDIO_TO_SB_MAILBOX: u32 = SDIO_BASE_ADDRESS + 0x40;
pub(crate) const SOCSRAM_BANKX_INDEX: u32 = SOCSRAM_BASE_ADDRESS + 0x10;
pub(crate) const SOCSRAM_BANKX_PDA: u32 = SOCSRAM_BASE_ADDRESS + 0x44;

pub(crate) const SBSDIO_ALP_AVAIL_REQ: u8 = 0x08;
pub(crate) const SBSDIO_HT_AVAIL_REQ: u8 = 0x10;
pub(crate) const SBSDIO_ALP_AVAIL: u8 = 0x40;
pub(crate) const SBSDIO_HT_AVAIL: u8 = 0x80;
pub(crate) const SBSDIO_FORCE_HW_CLKREQ_OFF: u8 = 0x20;
pub(crate) const SBSDIO_FORCE_ALP: u8 = 0x01;
pub(crate) const SBSDIO_FORCE_HT: u8 = 0x02;

pub(crate) const AI_IOCTRL_OFFSET: u32 = 0x408;
pub(crate) const SICF_CPUHALT: u32 = 0x0020;
pub(crate) const SICF_FGC: u32 = 0x0002;
pub(crate) const SICF_CLOCK_EN: u32 = 0x0001;
pub(crate) const AI_RESETCTRL_OFFSET: u32 = 0x800;
pub(crate) const AIRC_RESET: u32 = 1;

pub(crate) const SPI_F2_WATERMARK: u8 = 32;
pub(crate) const SDIO_F2_WATERMARK: u8 = 8;

pub(crate) const WWD_STA_INTERFACE: u32 = 0;
pub(crate) const WWD_AP_INTERFACE: u32 = 1;
pub(crate) const WWD_P2P_INTERFACE: u32 = 2;

pub(crate) const CONTROL_HEADER: u8 = 0;
pub(crate) const ASYNCEVENT_HEADER: u8 = 1;
pub(crate) const DATA_HEADER: u8 = 2;

pub(crate) const CDCF_IOC_ID_SHIFT: u32 = 16;
pub(crate) const CDCF_IOC_ID_MASK: u32 = 0xffff_0000;
pub(crate) const CDCF_IOC_IF_SHIFT: u32 = 12;

pub(crate) const SDPCM_GET: u32 = 0;
pub(crate) const SDPCM_SET: u32 = 2;

pub(crate) const WLC_UP: u32 = 2;
pub(crate) const WLC_SET_INFRA: u32 = 20;
pub(crate) const WLC_SET_AUTH: u32 = 22;
pub(crate) const WLC_GET_BSSID: u32 = 23;
pub(crate) const WLC_GET_SSID: u32 = 25;
pub(crate) const WLC_SET_SSID: u32 = 26;
pub(crate) const WLC_SET_CHANNEL: u32 = 30;
pub(crate) const WLC_DISASSOC: u32 = 52;
pub(crate) const WLC_GET_ANTDIV: u32 = 63;
pub(crate) const WLC_SET_ANTDIV: u32 = 64;
pub(crate) const WLC_SET_DTIMPRD: u32 = 78;
pub(crate) const WLC_GET_PM: u32 = 85;
pub(crate) const WLC_SET_PM: u32 = 86;
pub(crate) const WLC_SET_GMODE: u32 = 110;
pub(crate) const WLC_SET_WSEC: u32 = 134;
pub(crate) const WLC_SET_BAND: u32 = 142;
pub(crate) const WLC_GET_ASSOCLIST: u32 = 159;
pub(crate) const WLC_SET_WPA_AUTH: u32 = 165;
pub(crate) const WLC_GET_VAR: u32 = 262;
pub(crate) const WLC_SET_VAR: u32 = 263;
pub(crate) const WLC_SET_WSEC_PMK: u32 = 268;

pub(crate) const SDIOD_CCCR_IOEN: u32 = 0x02;
pub(crate) const SDIOD_CCCR_IORDY: u32 = 0x03;
pub(crate) const SDIOD_CCCR_INTEN: u32 = 0x04;
pub(crate) const SDIOD_CCCR_BICTRL: u32 = 0x07;
pub(crate) const SDIOD_CCCR_BLKSIZE_0: u32 = 0x10;
pub(crate) const SDIOD_CCCR_SPEED_CONTROL: u32 = 0x13;
pub(crate) const SDIOD_CCCR_BRCM_CARDCAP: u32 = 0xf0;
pub(crate) const SDIOD_SEP_INT_CTL: u32 = 0xf2;
pub(crate) const SDIOD_CCCR_F1BLKSIZE_0: u32 = 0x110;
pub(crate) const SDIOD_CCCR_F2BLKSIZE_0: u32 = 0x210;
pub(crate) const SDIOD_CCCR_F2BLKSIZE_1: u32 = 0x211;

pub(crate) const INTR_CTL_MASTER_EN: u32 = 0x01;
pub(crate) const INTR_CTL_FUNC1_EN: u32 = 0x02;
pub(crate) const INTR_CTL_FUNC2_EN: u32 = 0x04;

pub(crate) const SDIO_FUNC_ENABLE_1: u32 = 0x02;
pub(crate) const SDIO_FUNC_ENABLE_2: u32 = 0x04;
pub(crate) const SDIO_FUNC_READY_1: u32 = 0x02;
pub(crate) const SDIO_FUNC_READY_2: u32 = 0x04;
pub(crate) const SDIO_64B_BLOCK: u32 = 64;
pub(crate) const SDIO_PULL_UP: u32 = 0x1000f;

pub(crate) const SDIOD_CCCR_BRCM_CARDCAP_CMD14_SUPPORT: u32 = 0x02;
pub(crate) const SDIOD_CCCR_BRCM_CARDCAP_CMD14_EXT: u32 = 0x04;
pub(crate) const SDIOD_CCCR_BRCM_CARDCAP_CMD_NODEC: u32 = 0x08;

pub(crate) const SEP_INTR_CTL_MASK: u32 = 0x01;
pub(crate) const SEP_INTR_CTL_EN: u32 = 0x02;
pub(crate) const SEP_INTR_CTL_POL: u32 = 0x04;

pub(crate) const SBSDIO_WCTRL_WAKE_TILL_ALP_AVAIL: u32 = 1 << 0;
pub(crate) const SBSDIO_WCTRL_WAKE_TILL_HT_AVAIL: u32 = 1 << 1;

pub(crate) const SBSDIO_SLPCSR_KEEP_SDIO_ON: u32 = 1 << 0;
pub(crate) const SBSDIO_SLPCSR_DEVICE_ON: u32 = 1 << 1;

pub(crate) const DOT11_CAP_PRIVACY: u16 = 0x0010;
pub(crate) const DOT11_IE_ID_RSN: u8 = 48;
pub(crate) const DOT11_IE_ID_VENDOR_SPECIFIC: u8 = 221;
pub(crate) const WPA_OUI_TYPE1: &[u8; 4] = b"\x00\x50\xf2\x01";

// SPI

// Register addresses
pub(crate) const SPI_BUS_CONTROL: u32 = 0x0000;
pub(crate) const SPI_RESPONSE_DELAY: u32 = 0x0001;
pub(crate) const SPI_STATUS_ENABLE: u32 = 0x0002;
pub(crate) const SPI_RESET_BP: u32 = 0x0003;
pub(crate) const SPI_INTERRUPT_REGISTER: u32 = 0x0004;
pub(crate) const SPI_INTERRUPT_ENABLE_REGISTER: u32 = 0x0006;
pub(crate) const SPI_STATUS_REGISTER: u32 = 0x0008;
pub(crate) const SPI_FUNCTION1_INFO: u32 = 0x000C;
pub(crate) const SPI_FUNCTION2_INFO: u32 = 0x000E;
pub(crate) const SPI_FUNCTION3_INFO: u32 = 0x0010;
pub(crate) const SPI_READ_TEST_REGISTER: u32 = 0x0014;
pub(crate) const SPI_RESP_DELAY_F0: u32 = 0x001C;
pub(crate) const SPI_RESP_DELAY_F1: u32 = 0x001D;
pub(crate) const SPI_RESP_DELAY_F2: u32 = 0x001E;
pub(crate) const SPI_RESP_DELAY_F3: u32 = 0x001F;

// SPI_FUNCTIONX_BITS
pub(crate) const SPI_FUNCTIONX_ENABLED: u32 = 1 << 0;
pub(crate) const SPI_FUNCTIONX_READY: u32 = 1 << 1;

// SPI_BUS_CONTROL bits
pub(crate) const WORD_LENGTH_32: u32 = 0x01;
pub(crate) const ENDIAN_BIG: u32 = 0x02;
pub(crate) const CLOCK_PHASE: u32 = 0x04;
pub(crate) const CLOCK_POLARITY: u32 = 0x08;
pub(crate) const HIGH_SPEED_MODE: u32 = 0x10;
pub(crate) const INTERRUPT_POLARITY_HIGH: u32 = 0x20;
pub(crate) const WAKE_UP: u32 = 0x80;

// SPI_STATUS_ENABLE bits
pub(crate) const STATUS_ENABLE: u32 = 0x01;
pub(crate) const INTR_WITH_STATUS: u32 = 0x02;
pub(crate) const RESP_DELAY_ALL: u32 = 0x04;
pub(crate) const DWORD_PKT_LEN_EN: u32 = 0x08;
pub(crate) const CMD_ERR_CHK_EN: u32 = 0x20;
pub(crate) const DATA_ERR_CHK_EN: u32 = 0x40;

// SPI_INTERRUPT_REGISTER and SPI_INTERRUPT_ENABLE_REGISTER bits
pub(crate) const DATA_UNAVAILABLE: u16 = 0x0001;
pub(crate) const F2_F3_FIFO_RD_UNDERFLOW: u16 = 0x0002;
pub(crate) const F2_F3_FIFO_WR_OVERFLOW: u16 = 0x0004;
pub(crate) const COMMAND_ERROR: u16 = 0x0008;
pub(crate) const DATA_ERROR: u16 = 0x0010;
pub(crate) const F2_PACKET_AVAILABLE: u16 = 0x0020;
pub(crate) const F3_PACKET_AVAILABLE: u16 = 0x0040;
pub(crate) const F1_OVERFLOW: u16 = 0x0080;
pub(crate) const GSPI_PACKET_AVAILABLE: u16 = 0x0100;
pub(crate) const MISC_INTR1: u16 = 0x0200;
pub(crate) const MISC_INTR2: u16 = 0x0400;
pub(crate) const MISC_INTR3: u16 = 0x0800;
pub(crate) const MISC_INTR4: u16 = 0x1000;
pub(crate) const F1_INTR: u16 = 0x2000;
pub(crate) const F2_INTR: u16 = 0x4000;
pub(crate) const F3_INTR: u16 = 0x8000;

pub(crate) const BUS_OVERFLOW_UNDERFLOW: u16 =
    F1_OVERFLOW | F2_F3_FIFO_RD_UNDERFLOW | F2_F3_FIFO_WR_OVERFLOW;

// SPI_STATUS_REGISTER bits
pub(crate) const STATUS_DATA_NOT_AVAILABLE: u32 = 0x0000_0001;
pub(crate) const STATUS_UNDERFLOW: u32 = 0x0000_0002;
pub(crate) const STATUS_OVERFLOW: u32 = 0x0000_0004;
pub(crate) const STATUS_F2_INTR: u32 = 0x0000_0008;
pub(crate) const STATUS_F3_INTR: u32 = 0x0000_0010;
pub(crate) const STATUS_F2_RX_READY: u32 = 0x0000_0020;
pub(crate) const STATUS_F3_RX_READY: u32 = 0x0000_0040;
pub(crate) const STATUS_HOST_CMD_DATA_ERR: u32 = 0x0000_0080;
pub(crate) const STATUS_F2_PKT_AVAILABLE: u32 = 0x0000_0100;
pub(crate) const STATUS_F2_PKT_LEN_MASK: u32 = 0x000F_FE00;
pub(crate) const STATUS_F2_PKT_LEN_SHIFT: u32 = 9;
pub(crate) const STATUS_F3_PKT_AVAILABLE: u32 = 0x0010_0000;
pub(crate) const STATUS_F3_PKT_LEN_MASK: u32 = 0xFFE0_0000;
pub(crate) const STATUS_F3_PKT_LEN_SHIFT: u32 = 21;

pub(crate) const SPI_FRAME_CONTROL: u32 = 0x0001_000D;
