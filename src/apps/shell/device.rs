use crate::apps::shell::ShellApp;
use crate::println;
use crate::sys::device_driver;
use crate::sys::task::Priority;

const DEVICE_PRIO: u8 = 100;
const DEVICE_STACK_SIZE: usize = 256;

extern "C" fn devs_task(_arg: *mut ()) {
    let devices = device_driver::driver_manager().list_devices();

    println!("Register devices:");
    for (index, compatible) in devices.iter().enumerate() {
        println!("      {}. {}", index + 1, compatible);
    }
}

pub(super) static DEVS_APP: ShellApp = ShellApp::new(
    "devs",
    "Show devices information",
    devs_task,
    DEVICE_STACK_SIZE,
    Priority(DEVICE_PRIO),
);
