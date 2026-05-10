#![allow(dead_code)]

pub(super) mod ioctl {
    pub const GET_SSID: u32 = 0x32;
    pub const GET_CHANNEL: u32 = 0x3a;
    pub const SET_DISASSOC: u32 = 0x69;
    pub const GET_ANTDIV: u32 = 0x7e;
    pub const SET_ANTDIV: u32 = 0x81;
    pub const SET_MONITOR: u32 = 0xd9;
    pub const GET_RSSI: u32 = 0xfe;
    pub const GET_VAR: u32 = 0x20c;
    pub const SET_VAR: u32 = 0x20f;
}

pub(super) mod event {
    pub const SET_SSID: u32 = 0;
    pub const JOIN: u32 = 1;
    pub const AUTH: u32 = 3;
    pub const DEAUTH: u32 = 5;
    pub const DEAUTH_IND: u32 = 6;
    pub const ASSOC: u32 = 7;
    pub const DISASSOC: u32 = 11;
    pub const DISASSOC_IND: u32 = 12;
    pub const LINK: u32 = 16;
    pub const PRUNE: u32 = 23;
    pub const PSK_SUP: u32 = 46;
    pub const ICV_ERROR: u32 = 49;
    pub const ESCAN_RESULT: u32 = 69;
    pub const CSA_COMPLETE_IND: u32 = 80;
    pub const ASSOC_REQ_IE: u32 = 87;
    pub const ASSOC_RESP_IE: u32 = 88;
}

pub(super) mod status {
    pub const SUCCESS: u32 = 0;
    pub const FAIL: u32 = 1;
    pub const TIMEOUT: u32 = 2;
    pub const NO_NETWORKS: u32 = 3;
    pub const ABORT: u32 = 4;
    pub const NO_ACK: u32 = 5;
    pub const UNSOLICITED: u32 = 6;
    pub const ATTEMPT: u32 = 7;
    pub const PARTIAL: u32 = 8;
    pub const NEWSCAN: u32 = 9;
    pub const NEWASSOC: u32 = 10;
}

pub(super) mod sup {
    pub const DISCONNECTED: u32 = 0;
    pub const CONNECTING: u32 = 1;
    pub const IDREQUIRED: u32 = 2;
    pub const AUTHENTICATING: u32 = 3;
    pub const AUTHENTICATED: u32 = 4;
    pub const KEYXCHANGE: u32 = 5;
    pub const KEYED: u32 = 6;
    pub const TIMEOUT: u32 = 7;
    pub const LAST_BASIC_STATE: u32 = 8;
    pub const KEYXCHANGE_WAIT_M1: u32 = AUTHENTICATED;
    pub const KEYXCHANGE_PREP_M2: u32 = KEYXCHANGE;
    pub const KEYXCHANGE_WAIT_M3: u32 = LAST_BASIC_STATE;
    pub const KEYXCHANGE_PREP_M4: u32 = 9;
    pub const KEYXCHANGE_WAIT_G1: u32 = 10;
    pub const KEYXCHANGE_PREP_G2: u32 = 11;
}

pub(super) mod reason {
    pub const INITIAL_ASSOC: u32 = 0;
    pub const LOW_RSSI: u32 = 1;
    pub const DEAUTH: u32 = 2;
    pub const DISASSOC: u32 = 3;
    pub const BCNS_LOST: u32 = 4;
    pub const FAST_ROAM_FAILED: u32 = 5;
    pub const DIRECTED_ROAM: u32 = 6;
    pub const TSPEC_REJECTED: u32 = 7;
    pub const BETTER_AP: u32 = 8;

    pub const PRUNE_ENCR_MISMATCH: u32 = 1;
    pub const PRUNE_BCAST_BSSID: u32 = 2;
    pub const PRUNE_MAC_DENY: u32 = 3;
    pub const PRUNE_MAC_NA: u32 = 4;
    pub const PRUNE_REG_PASSV: u32 = 5;
    pub const PRUNE_SPCT_MGMT: u32 = 6;
    pub const PRUNE_RADAR: u32 = 7;
    pub const RSN_MISMATCH: u32 = 8;
    pub const PRUNE_NO_COMMON_RATES: u32 = 9;
    pub const PRUNE_BASIC_RATES: u32 = 10;
    pub const PRUNE_CCXFAST_PREVAP: u32 = 11;
    pub const PRUNE_CIPHER_NA: u32 = 12;
    pub const PRUNE_KNOWN_STA: u32 = 13;
    pub const PRUNE_CCXFAST_DROAM: u32 = 14;
    pub const PRUNE_WDS_PEER: u32 = 15;
    pub const PRUNE_QBSS_LOAD: u32 = 16;
    pub const PRUNE_HOME_AP: u32 = 17;
    pub const PRUNE_AP_BLOCKED: u32 = 18;
    pub const PRUNE_NO_DIAG_SUPPORT: u32 = 19;

    pub const SUP_OTHER: u32 = 0;
    pub const SUP_DECRYPT_KEY_DATA: u32 = 1;
    pub const SUP_BAD_UCAST_WEP128: u32 = 2;
    pub const SUP_BAD_UCAST_WEP40: u32 = 3;
    pub const SUP_UNSUP_KEY_LEN: u32 = 4;
    pub const SUP_PW_KEY_CIPHER: u32 = 5;
    pub const SUP_MSG3_TOO_MANY_IE: u32 = 6;
    pub const SUP_MSG3_IE_MISMATCH: u32 = 7;
    pub const SUP_NO_INSTALL_FLAG: u32 = 8;
    pub const SUP_MSG3_NO_GTK: u32 = 9;
    pub const SUP_GRP_KEY_CIPHER: u32 = 10;
    pub const SUP_GRP_MSG1_NO_GTK: u32 = 11;
    pub const SUP_GTK_DECRYPT_FAIL: u32 = 12;
    pub const SUP_SEND_FAIL: u32 = 13;
    pub const SUP_DEAUTH: u32 = 14;
    pub const SUP_WPA_PSK_TMO: u32 = 15;
}

pub(super) mod auth {
    pub const OPEN: u32 = 0;
    pub const WPA_TKIP_PSK: u32 = 0x0020_0002;
    pub const WPA2_AES_PSK: u32 = 0x0040_0004;
    pub const WPA2_MIXED_PSK: u32 = 0x0040_0006;
    pub const WPA3_SAE_AES_PSK: u32 = 0x0100_0004;
    pub const WPA3_WPA2_AES_PSK: u32 = 0x0140_0004;
}

pub(super) mod pm {
    pub const NO_POWERSAVE_MODE: u32 = 0;
    pub const PM1_POWERSAVE_MODE: u32 = 1;
    pub const PM2_POWERSAVE_MODE: u32 = 2;
}

// The maximum block size for transfers on the bus.
pub(super) const CYW43_BUS_MAX_BLOCK_SIZE: usize = 64;
pub(super) const CYW43_BACKPLANE_READ_PAD_LEN_BYTES: usize = 16;
