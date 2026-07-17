use crate::apps::shell::ShellApp;
use crate::gui::input;
use crate::println;
use crate::sys::console;
use crate::sys::syscall::sleep_ms;
use crate::sys::task::Priority;

const KEYBOARD_PRIO: u8 = 100;
const KEYBOARD_STACK_SIZE: usize = 256;

extern "C" fn keyboard_task(_arg: *mut ()) {
    println!("keyboard test started, press q to quit");

    input::input_manager::InputManager::pause(true);
    loop {
        match input::keyboard().read_key_value() {
            Ok(Some(key_value)) => {
                println!(
                    "Key state: {:?} Key Value: 0x{:02x}",
                    key_value.state, key_value.key
                );
            }

            Ok(None) => {}

            Err(e) => {
                println!("keyboard error: {:?}", e);
                break;
            }
        }
        sleep_ms(20);
        if let Some(c) = console::console().try_read_char() {
            if c == 'q' {
                break;
            }
        }
    }
    input::input_manager::InputManager::pause(false);
}

pub(super) static KEYBOARD_APP: ShellApp = ShellApp::new(
    "kbd",
    "Test keyboard function",
    keyboard_task,
    KEYBOARD_STACK_SIZE,
    Priority(KEYBOARD_PRIO),
);
