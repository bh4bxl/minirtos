use core::net::Ipv4Addr;

use crate::sys::{
    sync::Event,
    synchronization::{CriticalSectionLock, critical_section},
};

use super::{NetError, NetResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Config {
    pub address: Ipv4Addr,
    pub prefix_len: u8,
    pub gateway: Option<Ipv4Addr>,
    pub dns: Option<Ipv4Addr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkStatus {
    Down,
    Configuring,
    Configured(Ipv4Config),
}

static NETWORK_STATUS: CriticalSectionLock<NetworkStatus> =
    CriticalSectionLock::new(NetworkStatus::Down);

static NETWORK_STATE_CHANGED: Event = Event::new(false);

pub fn network_status() -> NetworkStatus {
    critical_section(|cs| NETWORK_STATUS.lock(cs, |status| *status))
}

pub fn network_config() -> Option<Ipv4Config> {
    match network_status() {
        NetworkStatus::Configured(config) => Some(config),
        NetworkStatus::Down | NetworkStatus::Configuring => None,
    }
}

pub fn wait_network() -> NetResult<Ipv4Config> {
    loop {
        match network_status() {
            NetworkStatus::Configured(config) => {
                return Ok(config);
            }

            NetworkStatus::Down => {
                return Err(NetError::NetworkDown);
            }

            NetworkStatus::Configuring => {
                NETWORK_STATE_CHANGED.wait();
            }
        }
    }
}

pub(crate) fn set_network_configuring() {
    set_status(NetworkStatus::Configuring);
}

pub(crate) fn set_network_config(config: Ipv4Config) {
    set_status(NetworkStatus::Configured(config));
}

pub(crate) fn set_network_down() {
    set_status(NetworkStatus::Down);
}

fn set_status(status: NetworkStatus) {
    critical_section(|cs| {
        NETWORK_STATUS.lock(cs, |current| {
            *current = status;
        });
    });

    /*
     * Event is only a wake-up notification.
     * The actual state is always read from NETWORK_STATUS.
     */
    NETWORK_STATE_CHANGED.signal();
}
