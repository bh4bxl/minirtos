use heapless::{Deque, Vec};
use smoltcp::wire::EthernetAddress;

use crate::{
    drivers::wlan::cyw43::cyw43_country::*,
    net::{
        PacketHandle, ScanResult, WifiAuth, WifiConnectFailure, WifiState, WlanPollResult, wlan,
    },
    sys::{
        device_driver::DevError,
        syscall::{self, sleep_ms},
    },
};

use super::*;

#[derive(Clone, Copy, Debug)]
pub enum WlanServiceEvent {
    LinkConnected,
    LinkDisconnected,
    ConnectFailed(WifiConnectFailure),
}

pub struct WlanService {
    mac: Option<EthernetAddress>,

    wifi_last_state: WifiState,
    wifi_is_on: bool,
    rx_buf: [u8; 1536],

    pending_tx: Option<PacketHandle>,
    pending_events: Deque<WlanServiceEvent, 8>,
    pending_connect_failure: Option<WifiConnectFailure>,
}

impl WlanService {
    pub fn new() -> Self {
        Self {
            mac: None,
            wifi_last_state: WifiState::Down,
            wifi_is_on: false,
            rx_buf: [0; 1536],
            pending_tx: None,
            pending_events: Deque::new(),
            pending_connect_failure: None,
        }
    }

    fn push_event(&mut self, event: WlanServiceEvent) {
        if self.pending_events.push_back(event).is_err() {
            defmt::warn!("WLANSRV: event queue full");
        }
    }

    pub fn take_event(&mut self) -> Option<WlanServiceEvent> {
        self.pending_events.pop_front()
    }

    pub fn wifi_on(&mut self) -> Result<(), NetworkError> {
        if self.wifi_is_on {
            defmt::info!("WLANSRV: wifi is already on");
            return Ok(());
        }

        defmt::info!("WLANSRV: wifi on");

        // clear
        self.mac = None;

        wlan().wifi_on(CYW43_COUNTRY_CANADA, None).map_err(|e| {
            defmt::warn!("WLANSRV: wifi on failed");
            NetworkError::Driver(e)
        })?;

        let mac = match wlan().get_mac_addr() {
            Ok(mac) => mac,
            Err(e) => {
                defmt::warn!("WLANSRV: get MAC address failed");

                // let _ = wlan().wifi_off();

                return Err(NetworkError::Driver(e));
            }
        };

        self.mac = Some(EthernetAddress(mac));

        defmt::info!(
            "WLANSRV: wlan mac: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0],
            mac[1],
            mac[2],
            mac[3],
            mac[4],
            mac[5],
        );

        self.wifi_is_on = true;

        for _ in 0..100 {
            match wlan().poll() {
                Ok(_) => {}
                Err(e) => {
                    defmt::warn!("WLANSRV: poll during startup failed: {:?}", e as usize);
                }
            }
            sleep_ms(10);
        }

        Ok(())
    }

    pub fn mac_addr(&self) -> Option<EthernetAddress> {
        self.mac
    }

    fn wifi_status(&self) -> Result<WifiState, NetworkError> {
        if !self.wifi_is_on {
            return Ok(WifiState::Down);
        }
        wlan().wifi_status().map_err(NetworkError::Driver)
    }

    pub fn wifi_scan(&mut self, timeout_ms: u32) -> Result<Vec<ScanResult, 32>, NetworkError> {
        if !self.wifi_is_on {
            defmt::warn!("WLANSRV: wifi is off");
            return Err(NetworkError::WifiOff);
        }

        defmt::info!("WLANSRV: wifi scan requested");

        wlan().wifi_scan().map_err(|e| {
            defmt::warn!("WLANSRV: wifi scan start failed");
            NetworkError::Driver(e)
        })?;

        let started_tick = syscall::get_tick();

        loop {
            wlan().poll().map_err(|e| {
                defmt::warn!("WLANSRV: poll during scan failed");
                NetworkError::Driver(e)
            })?;

            match wlan().wifi_scan_done() {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    defmt::warn!("WLANSRV: failed to query scan status");
                    return Err(NetworkError::Driver(e));
                }
            }

            let elapsed = syscall::get_tick().wrapping_sub(started_tick);

            if elapsed >= timeout_ms as u64 {
                defmt::warn!("WLANSRV: wifi scan timeout");
                return Err(NetworkError::Timeout);
            }

            sleep_ms(10);
        }

        let mut results = Vec::new();

        wlan().wifi_scan_results(&mut results).map_err(|e| {
            defmt::warn!("WLANSRV: failed to get scan results");
            NetworkError::Driver(e)
        })?;

        Ok(results)
    }

    pub fn wifi_connect(
        &mut self,
        ssid: FixedStr<32>,
        password: Option<FixedStr<64>>,
        auth: WifiAuth,
    ) -> Result<(), NetworkError> {
        match self.wifi_status()? {
            WifiState::Connected => {
                defmt::warn!("WLANSRV: already connected");
                return Ok(());
            }
            WifiState::Connecting => {
                defmt::warn!("WLANSRV: connection already in progress");
                return Err(NetworkError::Busy);
            }
            WifiState::Disconnecting => {
                defmt::warn!("WLANSRV: disconnection already in progress");
                return Err(NetworkError::Busy);
            }
            WifiState::Down => {}
        }

        defmt::info!("WLANSRV: wifi connect requested");

        let password = password.as_ref().map_or("", |pw| pw.as_str());

        wlan()
            .wifi_connect(ssid.as_str(), password, auth)
            .map_err(NetworkError::Driver)?;

        Ok(())
    }

    pub fn wifi_disconnect(&mut self) -> Result<(), NetworkError> {
        defmt::info!("WLANSRV: wifi disconnect requested");

        match self.wifi_status()? {
            WifiState::Down => {
                defmt::info!("WLANSRV: wifi is already disconnected");
            }
            _ => {
                wlan().wifi_disconnect().map_err(|e| {
                    defmt::warn!("WLANSRV: wifi disconnect failed");
                    NetworkError::Driver(e)
                })?;
            }
        }

        Ok(())
    }

    pub fn poll_rx(&mut self) {
        for _ in 0..16 {
            match self.poll_wifi() {
                Ok(true) => {}
                Ok(false) => break,
                Err(_) => {
                    defmt::warn!("WLANSRV: poll_wifi failed");
                    break;
                }
            }
        }
    }

    pub fn poll_state(&mut self) {
        self.poll_state_events();
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.wifi_status(), Ok(WifiState::Connected))
    }

    fn poll_state_events(&mut self) {
        match self.poll_wifi_state_change() {
            Ok(Some((old, new))) => {
                defmt::info!(
                    "WLANSRV: wifi state changed: {:?} -> {:?}",
                    old as usize,
                    new as usize
                );

                match new {
                    WifiState::Connected => {
                        self.push_event(WlanServiceEvent::LinkConnected);
                    }

                    WifiState::Down => {
                        if let Some(failure) = self.pending_connect_failure.take() {
                            self.push_event(WlanServiceEvent::ConnectFailed(failure));
                        } else if old == WifiState::Connected {
                            self.push_event(WlanServiceEvent::LinkDisconnected);
                        }
                    }

                    WifiState::Connecting | WifiState::Disconnecting => {}
                }
            }

            Ok(None) => {}

            Err(NetworkError::Driver(e)) => {
                defmt::warn!("WLANSRV: failed to poll wifi state: {}", e as usize);
            }

            Err(_) => {
                defmt::warn!("WLANSRV: failed to poll wifi state");
            }
        }
    }

    fn poll_wifi(&mut self) -> Result<bool, NetworkError> {
        match wlan().poll().map_err(NetworkError::Driver)? {
            WlanPollResult::Rx => {
                let len = wlan().get_rx_buf(&mut self.rx_buf).map_err(|e| {
                    defmt::warn!("WLANSRV: get_rx_buf failed: {:?}", e as usize);
                    NetworkError::Driver(e)
                })?;

                if len > self.rx_buf.len() {
                    defmt::warn!(
                        "WLANSRV: invalid RX length: {}, buffer size: {}",
                        len,
                        self.rx_buf.len()
                    );

                    return Err(NetworkError::InvalidPacket);
                }

                if !NETDEV.inject_rx(&self.rx_buf[..len]) {
                    defmt::warn!("WLANSRV: NETDEV inject_rx failed len={}", len);
                }

                Ok(true)
            }

            WlanPollResult::ConnectFailed(failure) => {
                match wlan().wifi_abort_connect() {
                    Ok(()) => {
                        self.pending_connect_failure = Some(failure);
                    }

                    Err(e) => {
                        defmt::warn!("WLANSRV: abort failed connection failed: {}", e as usize);

                        self.push_event(WlanServiceEvent::ConnectFailed(failure));
                    }
                }

                Ok(true)
            }

            WlanPollResult::None => Ok(false),
        }
    }

    fn poll_wifi_state_change(&mut self) -> Result<Option<(WifiState, WifiState)>, NetworkError> {
        let old = self.wifi_last_state;
        let new = self.wifi_status()?;

        if old == new {
            return Ok(None);
        }

        self.wifi_last_state = new;
        Ok(Some((old, new)))
    }

    pub fn drain_tx(&mut self) -> Result<(), NetworkError> {
        loop {
            let handle = match self.pending_tx.take() {
                Some(handle) => handle,
                None => match NETDEV.take_tx() {
                    Some(handle) => handle,
                    None => return Ok(()),
                },
            };

            let result = NETDEV.with_packet(handle, |pkt| wlan().sent_tx_buf(pkt));

            match result {
                Some(Ok(())) => {
                    NETDEV.free_packet(handle);
                }

                Some(Err(e)) if matches!(e, DevError::Busy | DevError::Timeout) => {
                    self.pending_tx = Some(handle);
                    return Ok(());
                }

                Some(Err(e)) => {
                    NETDEV.free_packet(handle);
                    return Err(NetworkError::Driver(e));
                }

                None => {
                    NETDEV.free_packet(handle);
                    return Err(NetworkError::InvalidPacket);
                }
            }
        }
    }
}
