#![allow(dead_code)]

pub(super) const SDIO_FUNCTION2_WATERMARK: u32 = 0x10008;
pub(super) const SDIO_BACKPLANE_ADDRESS_LOW: u32 = 0x1000a;
pub(super) const SDIO_BACKPLANE_ADDRESS_MID: u32 = 0x1000b;
pub(super) const SDIO_BACKPLANE_ADDRESS_HIGH: u32 = 0x1000c;
pub(super) const SDIO_CHIP_CLOCK_CSR: u32 = 0x1000e;
pub(super) const SDIO_WAKEUP_CTRL: u32 = 0x1001e;
pub(super) const SDIO_SLEEP_CSR: u32 = 0x1001f;

pub(super) const I_HMB_SW_MASK: u32 = 0x0000_00f0;
pub(super) const I_HMB_FC_CHANGE: u32 = 1 << 5;

pub(super) const CHIPCOMMON_BASE_ADDRESS: u32 = 0x1800_0000;
pub(super) const SDIO_BASE_ADDRESS: u32 = 0x1800_2000;
pub(super) const WLAN_ARMCM3_BASE_ADDRESS: u32 = 0x1800_3000;
pub(super) const SOCSRAM_BASE_ADDRESS: u32 = 0x1800_4000;
pub(super) const BACKPLANE_ADDR_MASK: u32 = 0x7fff;
pub(super) const WRAPPER_REGISTER_OFFSET: u32 = 0x10_0000;

pub(super) const SBSDIO_SB_ACCESS_2_4B_FLAG: u32 = 0x08000;

pub(super) const CHIPCOMMON_SR_CONTROL1: u32 = CHIPCOMMON_BASE_ADDRESS + 0x508;
pub(super) const SDIO_INT_STATUS: u32 = SDIO_BASE_ADDRESS + 0x20;
pub(super) const SDIO_INT_HOST_MASK: u32 = SDIO_BASE_ADDRESS + 0x24;
pub(super) const SDIO_FUNCTION_INT_MASK: u32 = SDIO_BASE_ADDRESS + 0x34;
pub(super) const SDIO_TO_SB_MAILBOX: u32 = SDIO_BASE_ADDRESS + 0x40;
pub(super) const SOCSRAM_BANKX_INDEX: u32 = SOCSRAM_BASE_ADDRESS + 0x10;
pub(super) const SOCSRAM_BANKX_PDA: u32 = SOCSRAM_BASE_ADDRESS + 0x44;

pub(super) const SBSDIO_ALP_AVAIL_REQ: u8 = 0x08;
pub(super) const SBSDIO_HT_AVAIL_REQ: u8 = 0x10;
pub(super) const SBSDIO_ALP_AVAIL: u8 = 0x40;
pub(super) const SBSDIO_HT_AVAIL: u8 = 0x80;
pub(super) const SBSDIO_FORCE_HW_CLKREQ_OFF: u8 = 0x20;
pub(super) const SBSDIO_FORCE_ALP: u8 = 0x01;
pub(super) const SBSDIO_FORCE_HT: u8 = 0x02;

pub(super) const AI_IOCTRL_OFFSET: u32 = 0x408;
pub(super) const SICF_CPUHALT: u32 = 0x0020;
pub(super) const SICF_FGC: u32 = 0x0002;
pub(super) const SICF_CLOCK_EN: u32 = 0x0001;
pub(super) const AI_RESETCTRL_OFFSET: u32 = 0x800;
pub(super) const AIRC_RESET: u32 = 1;

pub(super) const SPI_F2_WATERMARK: u8 = 32;
pub(super) const SDIO_F2_WATERMARK: u8 = 8;

pub(super) const WWD_STA_INTERFACE: u32 = 0;
pub(super) const WWD_AP_INTERFACE: u32 = 1;
pub(super) const WWD_P2P_INTERFACE: u32 = 2;

pub(super) const CONTROL_HEADER: u8 = 0;
pub(super) const ASYNCEVENT_HEADER: u8 = 1;
pub(super) const DATA_HEADER: u8 = 2;

pub(super) const CDCF_IOC_ID_SHIFT: u32 = 16;
pub(super) const CDCF_IOC_ID_MASK: u32 = 0xffff_0000;
pub(super) const CDCF_IOC_IF_SHIFT: u32 = 12;

pub(super) const SDPCM_GET: u32 = 0;
pub(super) const SDPCM_SET: u32 = 2;

pub(super) const WLC_UP: u32 = 2;
pub(super) const WLC_SET_INFRA: u32 = 20;
pub(super) const WLC_SET_AUTH: u32 = 22;
pub(super) const WLC_GET_BSSID: u32 = 23;
pub(super) const WLC_GET_SSID: u32 = 25;
pub(super) const WLC_SET_SSID: u32 = 26;
pub(super) const WLC_SET_CHANNEL: u32 = 30;
pub(super) const WLC_DISASSOC: u32 = 52;
pub(super) const WLC_GET_ANTDIV: u32 = 63;
pub(super) const WLC_SET_ANTDIV: u32 = 64;
pub(super) const WLC_SET_DTIMPRD: u32 = 78;
pub(super) const WLC_GET_PM: u32 = 85;
pub(super) const WLC_SET_PM: u32 = 86;
pub(super) const WLC_SET_GMODE: u32 = 110;
pub(super) const WLC_SET_WSEC: u32 = 134;
pub(super) const WLC_SET_BAND: u32 = 142;
pub(super) const WLC_GET_ASSOCLIST: u32 = 159;
pub(super) const WLC_SET_WPA_AUTH: u32 = 165;
pub(super) const WLC_GET_VAR: u32 = 262;
pub(super) const WLC_SET_VAR: u32 = 263;
pub(super) const WLC_SET_WSEC_PMK: u32 = 268;

pub(super) const SDIOD_CCCR_IOEN: u32 = 0x02;
pub(super) const SDIOD_CCCR_IORDY: u32 = 0x03;
pub(super) const SDIOD_CCCR_INTEN: u32 = 0x04;
pub(super) const SDIOD_CCCR_BICTRL: u32 = 0x07;
pub(super) const SDIOD_CCCR_BLKSIZE_0: u32 = 0x10;
pub(super) const SDIOD_CCCR_SPEED_CONTROL: u32 = 0x13;
pub(super) const SDIOD_CCCR_BRCM_CARDCAP: u32 = 0xf0;
pub(super) const SDIOD_SEP_INT_CTL: u32 = 0xf2;
pub(super) const SDIOD_CCCR_F1BLKSIZE_0: u32 = 0x110;
pub(super) const SDIOD_CCCR_F2BLKSIZE_0: u32 = 0x210;
pub(super) const SDIOD_CCCR_F2BLKSIZE_1: u32 = 0x211;

pub(super) const INTR_CTL_MASTER_EN: u32 = 0x01;
pub(super) const INTR_CTL_FUNC1_EN: u32 = 0x02;
pub(super) const INTR_CTL_FUNC2_EN: u32 = 0x04;

pub(super) const SDIO_FUNC_ENABLE_1: u32 = 0x02;
pub(super) const SDIO_FUNC_ENABLE_2: u32 = 0x04;
pub(super) const SDIO_FUNC_READY_1: u32 = 0x02;
pub(super) const SDIO_FUNC_READY_2: u32 = 0x04;
pub(super) const SDIO_64B_BLOCK: u32 = 64;
pub(super) const SDIO_PULL_UP: u32 = 0x1000f;

pub(super) const SDIOD_CCCR_BRCM_CARDCAP_CMD14_SUPPORT: u32 = 0x02;
pub(super) const SDIOD_CCCR_BRCM_CARDCAP_CMD14_EXT: u32 = 0x04;
pub(super) const SDIOD_CCCR_BRCM_CARDCAP_CMD_NODEC: u32 = 0x08;

pub(super) const SEP_INTR_CTL_MASK: u32 = 0x01;
pub(super) const SEP_INTR_CTL_EN: u32 = 0x02;
pub(super) const SEP_INTR_CTL_POL: u32 = 0x04;

pub(super) const SBSDIO_WCTRL_WAKE_TILL_ALP_AVAIL: u32 = 1 << 0;
pub(super) const SBSDIO_WCTRL_WAKE_TILL_HT_AVAIL: u32 = 1 << 1;

pub(super) const SBSDIO_SLPCSR_KEEP_SDIO_ON: u32 = 1 << 0;
pub(super) const SBSDIO_SLPCSR_DEVICE_ON: u32 = 1 << 1;

pub(super) const DOT11_CAP_PRIVACY: u16 = 0x0010;
pub(super) const DOT11_IE_ID_RSN: u8 = 48;
pub(super) const DOT11_IE_ID_VENDOR_SPECIFIC: u8 = 221;
pub(super) const WPA_OUI_TYPE1: &[u8; 4] = b"\x00\x50\xf2\x01";

// SPI

// Register addresses
pub(super) const SPI_BUS_CONTROL: u32 = 0x0000;
pub(super) const SPI_RESPONSE_DELAY: u32 = 0x0001;
pub(super) const SPI_STATUS_ENABLE: u32 = 0x0002;
pub(super) const SPI_RESET_BP: u32 = 0x0003;
pub(super) const SPI_INTERRUPT_REGISTER: u32 = 0x0004;
pub(super) const SPI_INTERRUPT_ENABLE_REGISTER: u32 = 0x0006;
pub(super) const SPI_STATUS_REGISTER: u32 = 0x0008;
pub(super) const SPI_FUNCTION1_INFO: u32 = 0x000C;
pub(super) const SPI_FUNCTION2_INFO: u32 = 0x000E;
pub(super) const SPI_FUNCTION3_INFO: u32 = 0x0010;
pub(super) const SPI_READ_TEST_REGISTER: u32 = 0x0014;
pub(super) const SPI_RESP_DELAY_F0: u32 = 0x001C;
pub(super) const SPI_RESP_DELAY_F1: u32 = 0x001D;
pub(super) const SPI_RESP_DELAY_F2: u32 = 0x001E;
pub(super) const SPI_RESP_DELAY_F3: u32 = 0x001F;

// SPI_FUNCTIONX_BITS
pub(super) const SPI_FUNCTIONX_ENABLED: u32 = 1 << 0;
pub(super) const SPI_FUNCTIONX_READY: u32 = 1 << 1;

// SPI_BUS_CONTROL bits
pub(super) const WORD_LENGTH_32: u32 = 0x01;
pub(super) const ENDIAN_BIG: u32 = 0x02;
pub(super) const CLOCK_PHASE: u32 = 0x04;
pub(super) const CLOCK_POLARITY: u32 = 0x08;
pub(super) const HIGH_SPEED_MODE: u32 = 0x10;
pub(super) const INTERRUPT_POLARITY_HIGH: u32 = 0x20;
pub(super) const WAKE_UP: u32 = 0x80;

// SPI_STATUS_ENABLE bits
pub(super) const STATUS_ENABLE: u32 = 0x01;
pub(super) const INTR_WITH_STATUS: u32 = 0x02;
pub(super) const RESP_DELAY_ALL: u32 = 0x04;
pub(super) const DWORD_PKT_LEN_EN: u32 = 0x08;
pub(super) const CMD_ERR_CHK_EN: u32 = 0x20;
pub(super) const DATA_ERR_CHK_EN: u32 = 0x40;

// SPI_INTERRUPT_REGISTER and SPI_INTERRUPT_ENABLE_REGISTER bits
pub(super) const DATA_UNAVAILABLE: u16 = 0x0001;
pub(super) const F2_F3_FIFO_RD_UNDERFLOW: u16 = 0x0002;
pub(super) const F2_F3_FIFO_WR_OVERFLOW: u16 = 0x0004;
pub(super) const COMMAND_ERROR: u16 = 0x0008;
pub(super) const DATA_ERROR: u16 = 0x0010;
pub(super) const F2_PACKET_AVAILABLE: u16 = 0x0020;
pub(super) const F3_PACKET_AVAILABLE: u16 = 0x0040;
pub(super) const F1_OVERFLOW: u16 = 0x0080;
pub(super) const GSPI_PACKET_AVAILABLE: u16 = 0x0100;
pub(super) const MISC_INTR1: u16 = 0x0200;
pub(super) const MISC_INTR2: u16 = 0x0400;
pub(super) const MISC_INTR3: u16 = 0x0800;
pub(super) const MISC_INTR4: u16 = 0x1000;
pub(super) const F1_INTR: u16 = 0x2000;
pub(super) const F2_INTR: u16 = 0x4000;
pub(super) const F3_INTR: u16 = 0x8000;

pub(super) const BUS_OVERFLOW_UNDERFLOW: u16 =
    F1_OVERFLOW | F2_F3_FIFO_RD_UNDERFLOW | F2_F3_FIFO_WR_OVERFLOW;

// SPI_STATUS_REGISTER bits
pub(super) const STATUS_DATA_NOT_AVAILABLE: u32 = 0x0000_0001;
pub(super) const STATUS_UNDERFLOW: u32 = 0x0000_0002;
pub(super) const STATUS_OVERFLOW: u32 = 0x0000_0004;
pub(super) const STATUS_F2_INTR: u32 = 0x0000_0008;
pub(super) const STATUS_F3_INTR: u32 = 0x0000_0010;
pub(super) const STATUS_F2_RX_READY: u32 = 0x0000_0020;
pub(super) const STATUS_F3_RX_READY: u32 = 0x0000_0040;
pub(super) const STATUS_HOST_CMD_DATA_ERR: u32 = 0x0000_0080;
pub(super) const STATUS_F2_PKT_AVAILABLE: u32 = 0x0000_0100;
pub(super) const STATUS_F2_PKT_LEN_MASK: u32 = 0x000F_FE00;
pub(super) const STATUS_F2_PKT_LEN_SHIFT: u32 = 9;
pub(super) const STATUS_F3_PKT_AVAILABLE: u32 = 0x0010_0000;
pub(super) const STATUS_F3_PKT_LEN_MASK: u32 = 0xFFE0_0000;
pub(super) const STATUS_F3_PKT_LEN_SHIFT: u32 = 21;

pub(super) const SPI_FRAME_CONTROL: u32 = 0x0001_000D;
