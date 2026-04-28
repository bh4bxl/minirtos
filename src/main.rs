#![no_std]
#![no_main]

use defmt_rtt as _;
use embedded_graphics::{
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use panic_probe as _;

use crate::{
    apps::shell::shell_task_entry,
    bsp::board_init,
    gui::display::FramebufferDisplay,
    sys::{
        arch::arm_cortex_m::start_first_task,
        device_driver::{self, DeviceType},
        sync::{message_queue::MessageQueue, mutex::Mutex, semaphore::Semaphore},
        syscall::{self, sleep_ms},
        task::Priority,
    },
};
use rp235x_hal as hal;

mod apps;
mod bsp;
mod drivers;
mod gui;
mod sys;

#[hal::entry]
fn main() -> ! {
    defmt::info!("MINI RTOS");

    match board_init() {
        Err(e) => defmt::error!("Error: {:?}", e as u16),
        Ok(()) => defmt::info!("Board {} initialized.", sys::board::board().board_name()),
    }

    m_info!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    sys::kernel_init().unwrap();

    syscall::thread_create(
        shell_task_entry,
        core::ptr::null_mut(),
        Priority(100),
        "shell",
    )
    .unwrap();

    syscall::thread_create(task1_entry, core::ptr::null_mut(), Priority(110), "task1").unwrap();

    syscall::thread_create(task2_entry, core::ptr::null_mut(), Priority(110), "task2").unwrap();

    unsafe {
        start_first_task();
    }
}

// Test tasks

fn trigger_gpio(pin: u8) {
    if let Some(gpio) = device_driver::driver_manager().open_device(DeviceType::Gpio, 0) {
        let mut data = [pin, 0];
        if let Err(_x) = gpio.read(&mut data) {
            defmt::error!("GPIO read failed: {}", data[0]);
        }

        data[1] = if data[1] == 0 { 1 } else { 0 };

        if let Err(_x) = gpio.write(&data) {
            defmt::error!("GPIO write failed: {}", data[0]);
        }
    }
}

// Test for Semaphore
// static TEST_SEM: Semaphore = Semaphore::new(0);

// Test for Mutex
// static M: Mutex = Mutex::new();

// Test for MessageQueue
static Q: MessageQueue<u32, 4> = MessageQueue::new();

extern "C" fn task1_entry(_: *mut ()) -> ! {
    let mut cnt = 0u32;
    loop {
        cnt += 1;
        defmt::info!("task1 running {}", cnt);
        // m_info!("task1 running {}", cnt);

        // For semaphore
        // m_info!("waiter: before wait");
        // TEST_SEM.wait();
        // m_info!("waiter: after wait");

        // For Mutex
        // M.lock();
        // print!("A ");
        // sleep_ms(500);
        // M.unlock();

        // For MessageQueue
        Q.send(cnt);
        defmt::info!("task1 send {}", cnt);
        sleep_ms(1000);

        trigger_gpio(19);

        // syscall::sleep_ms(1000);
    }
}

const LCD_W: usize = 240;
const LCD_H: usize = 135;
// const FB_SIZE: usize = 8 + LCD_W * LCD_H * 2;
// static mut SCREEN_BUFF: [u8; FB_SIZE] = [0; FB_SIZE];
const FB_SIZE: usize = LCD_W * LCD_H;
static mut SCREEN_BUFF: [u16; FB_SIZE] = [0; FB_SIZE];

extern "C" fn task2_entry(_: *mut ()) -> ! {
    let mut cnt = u32::MAX;
    loop {
        cnt -= 1;
        defmt::info!("task2 running {}", cnt);
        // m_info!("task2 running {}", cnt);

        // syscall::sleep_ms(1000);

        // For semaphore
        // m_info!("signaler: signal");
        // TEST_SEM.signal();

        // For Mutex
        // M.lock();
        // print!("B ");
        // sleep_ms(500);
        // M.unlock();

        // For MessageQueue
        let v = Q.recv();
        defmt::info!("task2 recv {}", v);

        // red 0xF800 green 0x07E0 blue 0x001F white 0xFFFF
        // let colors = [0xF800, 0x07E0, 0x001F, 0xFFFF];

        // let color: u16 = colors[cnt as usize % colors.len()];
        // let mut data = [0; 12];
        // data[1] = 0;
        // data[3] = 0;
        // data[5] = 240;
        // data[7] = 135;
        // for i in 0..FB_SIZE {
        //     if i % 2 == 0 {
        //         unsafe {
        //             SCREEN_BUFF[i] = (color >> 8) as u8;
        //         }
        //     } else {
        //         unsafe {
        //             SCREEN_BUFF[i] = color as u8;
        //         }
        //     }
        // }
        // let addr = (&raw const SCREEN_BUFF) as *const u8 as u32;
        // data[8..12].copy_from_slice(&addr.to_be_bytes());
        // let lcd = device_driver::driver_manager()
        //     .open_device(DeviceType::Lcd, 0)
        //     .unwrap();
        // lcd.write(&data).ok();

        let lcd = gui::lcd_flush();

        let mut display = unsafe {
            let ptr = &raw mut SCREEN_BUFF;
            let buf: &'static mut [u16; FB_SIZE] = &mut *ptr;
            FramebufferDisplay::new(lcd, buf)
        };

        display.clear_fb(Rgb565::BLACK);

        let colors = [Rgb565::RED, Rgb565::GREEN, Rgb565::YELLOW, Rgb565::BLUE];

        Rectangle::new(Point::new(0, 0), Size::new(240, 135))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(&mut display)
            .ok();

        Rectangle::new(Point::new(20, 20), Size::new(80, 40))
            .into_styled(PrimitiveStyle::with_fill(
                colors[cnt as usize % colors.len()],
            ))
            .draw(&mut display)
            .ok();

        let style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);

        let text_pos_x = 20 + ((v as i32) % 10) * 10;
        Text::new("miniRTOS V0.1", Point::new(text_pos_x, 80), style)
            .draw(&mut display)
            .ok();
        display.flush();

        trigger_gpio(21);

        // Test for HardFault
        // unsafe {
        //     core::ptr::write_volatile(0xFFFF_FFFC as *mut u32, 0x1234_5678);
        // }

        // syscall::sleep_ms(2000);
    }
}
