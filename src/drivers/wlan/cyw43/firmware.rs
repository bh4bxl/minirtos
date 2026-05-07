pub(super) static CYW43_FW: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/w43439A0_7_95_49_00_combined.bin"
));
pub(super) const CYW43_FW_LEN: usize = 224190;

pub(super) static WIFI_NVRAM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/wifi_nvram_43439.bin"));
pub(super) const WIFI_NVRAM_LEN: usize = 984;
