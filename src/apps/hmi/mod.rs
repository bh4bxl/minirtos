use crate::sys::{input, syscall, task::Priority};

pub mod ui;

const HMI_PRIO: u8 = 100;

pub fn start_hmi() -> Result<(), &'static str> {
    if let Err(x) = syscall::thread_create(
        hmi_task_entry,
        core::ptr::null_mut(),
        Priority(HMI_PRIO),
        "hmi",
    ) {
        Err(x)
    } else {
        Ok(())
    }
}

extern "C" fn hmi_task_entry(_: *mut ()) -> ! {
    ui::main_windows();

    loop {
        while let Some(event) = input::input_queue().poll_event() {
            match event {
                input::InputEvent::KeyDown(key) => {
                    defmt::info!("Key pressed: {:?}", key as u32);
                    // ui.handle_input(event);
                    // ui.draw();
                }
                input::InputEvent::KeyUp(key) => {
                    defmt::info!("Key released: {:?}", key as u32);
                }
            }
        }

        syscall::sleep_ms(20);
    }
}
