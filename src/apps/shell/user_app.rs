use crate::apps::shell::ShellApp;
use crate::sys::task::{Priority, Privilege};

const USER_PRIO: u8 = 100;
const USER_STACK_SIZE: usize = 256;
static mut USER_CONTROL: u32 = 0;

extern "C" fn user_app_task(_arg: *mut ()) {
    let control: u32;

    unsafe {
        core::arch::asm!(
            "mrs {0}, CONTROL",
            out(reg) control,
            options(nomem, nostack, preserves_flags),
        );

        USER_CONTROL = control;
    }
}

pub(super) static USER_APP: ShellApp = ShellApp::new(
    "user_app",
    "An Unprivileged App",
    user_app_task,
    USER_STACK_SIZE,
    Priority(USER_PRIO),
    Privilege::Unprivileged,
);
