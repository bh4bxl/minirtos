use crate::apps::shell::ShellApp;

use crate::sys::syscall;
use crate::sys::task::{Priority, Privilege};

const USER_PRIO: u8 = 100;
const USER_STACK_SIZE: usize = 256;

extern "C" fn user_app_task(_arg: *mut ()) {
    for i in 0..10 {
        crate::print!("{}..", i);
        syscall::sleep_ms(1000);
    }
    crate::println!("10");

    crate::print!("Input: ");

    let mut buf = [0u8; 64];
    let c = syscall::read_line(&mut buf);

    crate::println!("{}", c);
}

pub(super) static USER_APP: ShellApp = ShellApp::new(
    "user_app",
    "An Unprivileged App",
    user_app_task,
    USER_STACK_SIZE,
    Priority(USER_PRIO),
    Privilege::Unprivileged,
);
