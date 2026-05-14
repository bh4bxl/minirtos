use heapless::Vec;
use rp235x_pac::{self as pac};

use crate::{
    drivers::gpio,
    net::{ScanResult, WifiAuth, WifiState, interface::Wlan},
    sys::{
        device_driver::{self, DevError},
        interrupt,
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

pub mod cyw43_bus;
pub mod cyw43_consts;
pub mod cyw43_country;
pub mod cyw43_fw;
pub mod cyw43_inner;
pub mod cyw43_ioctl;
pub mod cyw43_regs;
pub mod cyw43_sdpcm;
pub mod pio_ctrl;
pub mod pio_spi;

#[allow(dead_code)]
struct Cyw43Inner {
    gpio: &'static dyn gpio::interface::Gpio,

    wl_dio: gpio::Pin,
    wl_cs: gpio::Pin,
    wl_on: gpio::Pin,

    bus: cyw43_bus::Cyw43Bus,

    sdpcm_seq: u8,
    packet_tx_seq: u8,
    last_bus_data_credit: u8,
    wlan_flow_control: u8,
    requested_ioctl_id: u8,
    had_successful_packet: bool,
    spid_buf: [u8; 2048],
    startup_t0: u64,

    scan_results: Vec<ScanResult, 32>,
    scan_done: bool,
    scan_in_progress: bool,

    bus_is_up: bool,
    state: WifiState,
}

// Utils

pub(super) fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

pub(super) fn ticks_us() -> u64 {
    let timer = unsafe { &*pac::TIMER0::ptr() };

    loop {
        let hi1 = timer.timerawh().read().bits();
        let lo = timer.timerawl().read().bits();
        let hi2 = timer.timerawh().read().bits();

        if hi1 == hi2 {
            return ((hi1 as u64) << 32) | lo as u64;
        }
    }
}

pub struct Cyw43 {
    inner: IrqSafeNullLock<Cyw43Inner>,
}

impl Cyw43 {
    pub const COMPATIBLE: &'static str = "CYW43439 Wlan";

    pub const fn new(
        gpio: &'static dyn gpio::interface::Gpio,
        wl_clk: usize,
        wl_dio: usize,
        wl_cs: usize,
        wl_on: usize,
    ) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Cyw43Inner::new(gpio, wl_clk, wl_dio, wl_cs, wl_on)),
        }
    }

    pub fn init_hw(&self, pio0: pac::PIO0, resets: &mut pac::RESETS) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init_hw(pio0, resets))
    }
}

impl device_driver::interface::Driver for Cyw43 {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    fn init(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.init())
    }

    fn register_irq_handler(
        &'static self,
        _irq_number: Self::IrqNumberType,
    ) -> Result<(), &'static str> {
        Ok(())
    }
}

static mut GPIO_LEVEL: bool = false;

impl device_driver::interface::Device for Cyw43 {
    fn read(&self, _data: &mut [u8]) -> Result<usize, DevError> {
        Err(DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, DevError> {
        self.inner.lock(|inner| {
            //inner.ensure_up()?;
            inner.wifi_on(cyw43_country::CYW43_COUNTRY_CANADA)?;
            inner.wifi_scan()?;
            unsafe { GPIO_LEVEL = !GPIO_LEVEL };
            inner.gpio_set(0, unsafe { GPIO_LEVEL })?;

            Ok(4)
        })
    }
}

impl device_driver::interface::DeviceDriver for Cyw43 {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}

impl interrupt::interface::IrqHandler for Cyw43 {
    fn handler(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl Wlan for Cyw43 {
    fn wifi_on(&self, country: u32, _mac: Option<[u8; 6]>) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.wifi_on(country))
    }

    fn wifi_scan(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.wifi_scan())
    }

    fn poll(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.poll())
    }

    fn wifi_scan_done(&self) -> Result<bool, DevError> {
        self.inner.lock(|inner| Ok(inner.scan_done))
    }

    fn wifi_scan_results(&self, res: &mut heapless::Vec<ScanResult, 32>) -> Result<(), DevError> {
        self.inner.lock(|inner| {
            res.clear();

            for r in inner.scan_results.iter() {
                res.push(*r).ok();
            }

            Ok(())
        })
    }

    fn wifi_connect(&self, _ssid: &str, _password: &str, auth: WifiAuth) -> Result<(), DevError> {
        self.inner
            .lock(|inner| inner.wifi_connect(_ssid, _password, auth))
    }

    fn wifi_status(&self) -> Result<WifiState, DevError> {
        self.inner.lock(|inner| Ok(inner.get_state()))
    }

    fn wifi_gpio_ctrl(&self, pin: usize, level: bool) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.gpio_set(pin, level))
    }

    fn wifi_disconnect(&self) -> Result<(), DevError> {
        self.inner.lock(|inner| inner.wifi_disconnect())
    }

    fn wifi_off(&self) -> Result<(), DevError> {
        Err(DevError::Unsupported)
    }
}
