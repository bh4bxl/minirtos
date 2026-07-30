#![allow(dead_code)]
use crate::{
    net::interface::Wlan,
    sys::{
        device_driver::DevError,
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

mod ethernet;
mod iface;
mod net_device;
mod net_stack;
mod packet;

pub use net_device::NetDevice;
pub use net_stack::NetStack;
pub use packet::PacketHandle;

#[derive(Clone, Copy, Debug)]
pub struct ScanResult {
    pub ssid: [u8; 32],
    pub ssid_len: usize,
    pub bssid: [u8; 6],
    pub rssi: i16,
    pub channel: u16,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiAuth {
    Open,
    WpaTkipPsk,
    Wpa2AesPsk,
    Wpa2MixedPsk,
    Wpa3SaeAesPsk,
    Wpa3Wpa2AesPsk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiState {
    Down,
    Connecting,
    Connected,
    Disconnecting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiConnectFailure {
    AuthFailed {
        status: u32,
        reason: u32,
    },
    AssocFailed {
        status: u32,
        reason: u32,
    },
    SetSsidFailed {
        status: u32,
        reason: u32,
    },
    PskFailed {
        status: u32,
        reason: u32,
    },
    Pruned {
        status: u32,
        reason: u32,
    },
    Deauthenticated {
        reason: u32,
    },
    Disassociated {
        reason: u32,
    },
    Timeout,
    Unknown {
        event: u32,
        status: u32,
        reason: u32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WlanPollResult {
    None,
    Rx,
    ConnectFailed(WifiConnectFailure),
}

#[allow(dead_code)]
pub mod interface {

    use crate::sys::device_driver::DevError;

    pub trait Wlan {
        fn wifi_on(&self, country: u32, mac: Option<[u8; 6]>) -> Result<(), DevError>;

        fn wifi_scan(&self) -> Result<(), DevError>;

        fn poll(&self) -> Result<super::WlanPollResult, DevError>;

        fn wifi_scan_done(&self) -> Result<bool, DevError>;

        fn wifi_scan_results(
            &self,
            _res: &mut heapless::Vec<super::ScanResult, 32>,
        ) -> Result<(), DevError> {
            Err(DevError::Unsupported)
        }

        fn wifi_connect(
            &self,
            ssid: &str,
            password: &str,
            auth: super::WifiAuth,
        ) -> Result<(), DevError>;

        fn wifi_status(&self) -> Result<super::WifiState, DevError> {
            Err(DevError::Unsupported)
        }

        fn get_mac_addr(&self) -> Result<[u8; 6], DevError> {
            Err(DevError::Unsupported)
        }

        fn get_rx_buf(&self, _rx_buf: &mut [u8]) -> Result<usize, DevError> {
            Err(DevError::Unsupported)
        }

        fn sent_tx_buf(&self, _tx_buf: &[u8]) -> Result<(), DevError> {
            Err(DevError::Unsupported)
        }

        fn wifi_gpio_ctrl(&self, _pin: usize, _level: bool) -> Result<(), DevError> {
            Err(DevError::Unsupported)
        }

        fn wifi_disconnect(&self) -> Result<(), DevError>;

        fn wifi_abort_connect(&self) -> Result<(), DevError>;

        fn wifi_off(&self) -> Result<(), DevError>;
    }
}

/// A placeholder.
struct NullWlan;

impl Wlan for NullWlan {
    fn wifi_on(&self, _country: u32, _mac: Option<[u8; 6]>) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_scan(&self) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn poll(&self) -> Result<WlanPollResult, DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_scan_done(&self) -> Result<bool, DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_scan_results(&self, _res: &mut heapless::Vec<ScanResult, 32>) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_connect(&self, _ssid: &str, _password: &str, _auth: WifiAuth) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_disconnect(&self) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_abort_connect(&self) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }

    fn wifi_off(&self) -> Result<(), DevError> {
        Err(DevError::NoSuchDevice)
    }
}

static NULL_WLAN: NullWlan = NullWlan {};

/// A reference to the global console.
static CURR_WLAN: IrqSafeNullLock<&'static (dyn interface::Wlan + Sync)> =
    IrqSafeNullLock::new(&NULL_WLAN);

/// Register a new console.
pub fn register_wlan(new_console: &'static (dyn interface::Wlan + Sync)) {
    CURR_WLAN.lock(|con| *con = new_console);
}

/// Return a reference to the currently registered console.
pub fn wlan() -> &'static dyn interface::Wlan {
    CURR_WLAN.lock(|con| *con)
}
