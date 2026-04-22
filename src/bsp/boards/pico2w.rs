use core::sync::atomic::{AtomicBool, Ordering};

use rp235x_hal::{self as hal, Watchdog, clocks, pac};

use crate::{
    bsp::mcu::rp235x::rp235x_interrupt::Rp235xIrqManger,
    drivers::{self, gpio::Gpio, uart::interface::Uart},
    sys::{board, console, device_driver, interrupt::register_irq_manager},
};

static IRQ_MANAGER: Rp235xIrqManger = Rp235xIrqManger::new();

static GPIO: drivers::gpio::rp235x_gpio::Rp235xGpio = drivers::gpio::rp235x_gpio::Rp235xGpio::new();

fn gpio_config() -> Result<(), &'static str> {
    // Uart pins;
    let (uart_tx, uart_rx) = (drivers::gpio::Pin(0), drivers::gpio::Pin(1));
    GPIO.eable(&uart_tx, true);
    GPIO.eable(&uart_rx, true);
    GPIO.set_function(&uart_tx, drivers::gpio::Function::UART);
    GPIO.set_pull(&uart_tx, drivers::gpio::Pull::None);
    GPIO.set_direction(&uart_tx, drivers::gpio::Direction::Output, true);
    GPIO.set_function(&uart_rx, drivers::gpio::Function::UART);
    GPIO.set_direction(&uart_rx, drivers::gpio::Direction::Input, true);
    GPIO.set_pull(&uart_rx, drivers::gpio::Pull::Up);

    // Test Pin
    let test_pin = drivers::gpio::Pin(19);
    GPIO.eable(&test_pin, true);
    GPIO.set_function(&test_pin, drivers::gpio::Function::SIO);
    GPIO.set_pull(&test_pin, drivers::gpio::Pull::None);
    GPIO.set_direction(&test_pin, drivers::gpio::Direction::Output, true);
    GPIO.set_level(&test_pin, drivers::gpio::Level::Low);

    Ok(())
}

fn gpio_register() -> Result<(), &'static str> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &GPIO,
        Some(gpio_config),
        None,
        device_driver::DeviceType::Gpio,
    );
    device_driver::driver_manager().register(descriptor);

    Ok(())
}

static UART0: drivers::uart::rp235x_pl011_uart::Pl011Uart =
    drivers::uart::rp235x_pl011_uart::Pl011Uart::new(
        drivers::uart::rp235x_pl011_uart::UartId::UART0,
    );

fn uart_config() -> Result<(), &'static str> {
    let config = drivers::uart::Config::default();
    UART0.config(&config);

    console::register_console(&UART0);

    Ok(())
}

fn uart_register() -> Result<(), &'static str> {
    let descriptor = device_driver::DeviceDriverDescriptor::new(
        &UART0,
        Some(uart_config),
        Some(rp235x_pac::Interrupt::UART0_IRQ),
        device_driver::DeviceType::Uart,
    );
    device_driver::driver_manager().register(descriptor);

    Ok(())
}

pub struct Pico2wBoard;

impl board::interface::Info for Pico2wBoard {
    fn board_name(&self) -> &'static str {
        "Raspberry Pico 2W"
    }
}

impl board::interface::All for Pico2wBoard {}

static PICO2W_BOARD: Pico2wBoard = Pico2wBoard {};

fn init_clock() -> Result<(), &'static str> {
    let mut pac = pac::Peripherals::take().unwrap();

    // clocks
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let _clocks = clocks::init_clocks_and_plls(
        12_000_000,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    );
    Ok(())
}

pub fn board_init() -> Result<(), &'static str> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err("Init already done");
    }

    init_clock()?;

    register_irq_manager(&IRQ_MANAGER);

    gpio_register()?;

    uart_register()?;

    board::register_board(&PICO2W_BOARD);

    Ok(())
}

/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    rp235x_hal::binary_info::rp_cargo_bin_name!(),
    rp235x_hal::binary_info::rp_cargo_version!(),
    rp235x_hal::binary_info::rp_program_description!(c"RP2350 miniRTOS"),
    rp235x_hal::binary_info::rp_cargo_homepage_url!(),
    rp235x_hal::binary_info::rp_program_build_attribute!(),
];
