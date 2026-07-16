#![allow(dead_code)]
use crate::sys::{
    SysError, input, syscall,
    task::{Priority, Task},
};

pub mod ui;

const HMI_PRIO: u8 = 100;

const HMI_STACK_SIZE: usize = 8196;

pub fn start_hmi() -> Result<(), SysError> {
    let mut hmi = Task::<HMI_STACK_SIZE>::new(hmi_task_entry)
        .priority(Priority(HMI_PRIO))
        .name("hmi");
    hmi.run()?;

    Ok(())
}

extern "C" fn hmi_task_entry(_: *mut ()) {
    ui::desktop();

    loop {
        let mut redraw = false;
        while let Some(event) = input::input_queue().poll_event() {
            match event {
                input::InputEvent::KeyDown(key) => {
                    defmt::info!("Key pressed: {:?}", key as u32);
                }
                input::InputEvent::KeyUp(key) => {
                    defmt::info!("Key released: {:?}", key as u32);
                }
                _ => {}
            }

            redraw = true;
        }

        if redraw {
            ui::desktop();
        }

        syscall::sleep_ms(20);
    }
}
