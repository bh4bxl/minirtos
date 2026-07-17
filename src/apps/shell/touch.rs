use crate::apps::shell::ShellApp;
use crate::gui::input;
use crate::println;
use crate::sys::console;
use crate::sys::syscall::sleep_ms;
use crate::sys::task::Priority;

const TOUCH_PRIO: u8 = 100;
const TOUCH_STACK_SIZE: usize = 256;

extern "C" fn touch_task(_arg: *mut ()) {
    println!("touch test started, press q to quit");

    input::input_manager::InputManager::pause(true);
    loop {
        match input::touch().read_point() {
            Ok(Some(report)) => {
                for i in 0..report.count {
                    let p = report.points[i];

                    println!("point{} id={} x={} y={} size={}", i, p.id, p.x, p.y, p.size);
                }
            }

            Ok(None) => {}

            Err(e) => {
                println!("touch error: {:?}", e);
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

pub(super) static TOUCH_APP: ShellApp = ShellApp::new(
    "touch",
    "Test touch function",
    touch_task,
    TOUCH_STACK_SIZE,
    Priority(TOUCH_PRIO),
);
