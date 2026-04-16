pub mod rp235x_gpio;

pub struct Pin(pub usize);

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Function {
    XIP = 0,
    SPI = 1,
    UART = 2,
    I2C = 3,
    PWM = 4,
    SIO = 5,
    PIO0 = 6,
    PIO1 = 7,
    CLOCK = 8,
    USB = 9,
    NULL = 0x1f,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Pull {
    Up,
    Down,
    None,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    Input,
    Output,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Level {
    High,
    Low,
}

#[allow(dead_code)]
pub trait Gpio {
    fn eable(&self, pin: &Pin, enable: bool);

    fn set_function(&self, pin: &Pin, func: Function);

    fn set_pull(&self, pin: &Pin, pull: Pull);

    fn set_direction(&self, pin: &Pin, direction: Direction, enable: bool);

    fn set_level(&self, pin: &Pin, level: Level);

    fn get_level(&self, pin: &Pin) -> Level;
}
