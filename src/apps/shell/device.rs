use crate::println;
use crate::sys::task::{Priority, Task};
use crate::sys::{SysError, device_driver};

const DEVICE_PRIO: u8 = 100;
const DEVICE_STACK_SIZE: usize = 256;

pub fn start_dev() -> Result<(), SysError> {
    let mut shell = Task::<DEVICE_STACK_SIZE>::new(devs_task)
        .priority(Priority(DEVICE_PRIO))
        .name("shell");

    shell.run()?;

    Ok(())
}

extern "C" fn devs_task(_arg: *mut ()) {
    let devices = device_driver::driver_manager().list_devices();

    for (index, compatible) in devices.iter().enumerate() {
        println!("      {}. {}", index + 1, compatible);
    }

    //crate::sys::syscall::task_exit()
}
