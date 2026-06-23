use core::sync::atomic::{AtomicBool, Ordering};

use crate::sys::{
    self, SysError,
    input::Key,
    syscall,
    task::{Priority, Task},
};

const INPUT_MGR_PRIO: u8 = 100;
const INPUT_MGR_STACK_SIZE: usize = 1024;

static INPUT_PAUSED: AtomicBool = AtomicBool::new(false);
pub struct InputManager;

impl InputManager {
    extern "C" fn task(_arg: *mut ()) -> ! {
        loop {
            Self::poll_keyboard();

            Self::poll_touch();

            syscall::sleep_ms(20);
        }
    }

    fn poll_keyboard() {
        match super::keyboard().read_key_value() {
            Ok(Some(raw_value)) => {
                let key = match raw_value.key {
                    0x61 | 0x41 => Key::A,
                    0x62 | 0x42 => Key::B,
                    0xb4 => Key::Left,
                    0xb5 => Key::Up,
                    0xb6 => Key::Down,
                    0xb7 => Key::Right,
                    _ => Key::None,
                };
                let key_event = match raw_value.state {
                    super::KeyState::Pressed => sys::input::InputEvent::KeyDown(key),
                    super::KeyState::Released => sys::input::InputEvent::KeyUp(key),
                    super::KeyState::Hold => sys::input::InputEvent::KeyHold(key),
                    super::KeyState::Idle => sys::input::InputEvent::None,
                };
                let _ = sys::input::input_queue().push_event(key_event);
            }

            Ok(None) => {}

            Err(_) => {}
        }
    }

    fn poll_touch() {}

    pub fn start() -> Result<(), SysError> {
        let mut input_mgr = Task::<INPUT_MGR_STACK_SIZE>::new(Self::task)
            .priority(Priority(INPUT_MGR_PRIO))
            .name("input_mgr");

        input_mgr.run()?;

        Ok(())
    }

    pub fn pause(pause: bool) {
        INPUT_PAUSED.store(pause, Ordering::SeqCst);
    }
}
