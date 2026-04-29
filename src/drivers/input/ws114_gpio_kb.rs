use crate::{
    drivers::gpio::{self, Level, Pin},
    sys::{
        device_driver,
        input::{self, Key},
        synchronization::{IrqSafeNullLock, interface::Mutex},
    },
};

const MAX_KEYS: usize = 7;

const KEYS_MAP: [(u32, Key); MAX_KEYS] = [
    (2, Key::Up),
    (3, Key::Enter),
    (15, Key::A),
    (16, Key::Left),
    (17, Key::B),
    (18, Key::Down),
    (20, Key::Right),
];

/// GPIO IRQ Handler
fn keyboard_gpio_irq_handler(pin: &Pin, level: Level) {
    let mut key = Key::None;
    for i in 0..MAX_KEYS {
        if KEYS_MAP[i].0 == pin.0 as u32 {
            key = KEYS_MAP[i].1;
            break;
        }
    }
    if key == Key::None {
        return;
    }

    let key_event = match level {
        Level::High => input::InputEvent::KeyUp(key),
        Level::Low => input::InputEvent::KeyDown(key),
    };
    input::input_queue().push_event(key_event).unwrap();
}

struct Ws114UartKeyboardInner {
    gpio: &'static dyn gpio::interface::Gpio,
}

impl Ws114UartKeyboardInner {
    pub const fn new(gpio: &'static dyn gpio::interface::Gpio) -> Self {
        Self { gpio }
    }

    fn init(&self) -> Result<(), device_driver::DevError> {
        for (key, _) in KEYS_MAP.iter() {
            let pin = Pin(*key as usize);

            self.gpio
                .register_irq_handler(&pin, Some(keyboard_gpio_irq_handler));
        }
        Ok(())
    }
}

pub struct Ws114GpioKeyboard {
    inner: IrqSafeNullLock<Ws114UartKeyboardInner>,
}

impl Ws114GpioKeyboard {
    pub const COMPATIBLE: &'static str = "SW114 GPIO Buttons";

    pub const fn new(gpio: &'static dyn gpio::interface::Gpio) -> Self {
        Self {
            inner: IrqSafeNullLock::new(Ws114UartKeyboardInner::new(gpio)),
        }
    }
}

impl device_driver::interface::Driver for Ws114GpioKeyboard {
    type IrqNumberType = rp235x_pac::Interrupt;

    fn init(&self) -> Result<(), device_driver::DevError> {
        self.inner.lock(|inner| inner.init())
    }

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }
}

impl device_driver::interface::Device for Ws114GpioKeyboard {
    fn read(&self, _data: &mut [u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }

    fn write(&self, _data: &[u8]) -> Result<usize, device_driver::DevError> {
        Err(device_driver::DevError::Unsupported)
    }
}

impl device_driver::interface::DeviceDriver for Ws114GpioKeyboard {
    fn as_device(&self) -> &dyn device_driver::interface::Device {
        self
    }
}
