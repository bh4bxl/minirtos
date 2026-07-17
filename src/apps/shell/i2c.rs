use crate::apps::shell::ShellApp;
use crate::println;
use crate::sys::device_driver;
use crate::sys::task::Priority;

const I2C_PRIO: u8 = 100;
const I2C_STACK_SIZE: usize = 256;

extern "C" fn i2c_task(arg: *mut ()) {
    let context = unsafe { super::take_context(arg) };

    match context.arg(0) {
        Some("scan") => {
            if context.argc() != 1 {
                println!("usage: i2c scan");
                return;
            }

            println!("i2c scan:");

            let Ok(dev) =
                device_driver::driver_manager().open_device(device_driver::DeviceType::I2c, 0)
            else {
                println!("Cannot find i2c dev");
                return;
            };

            for addr in 0x08u8..0x77 {
                let mut buf = [0u8; 2];
                buf[0] = addr;

                if dev.read(&mut buf).is_ok() {
                    println!("  found: 0x{:02x}", addr);
                }
            }
        }

        Some("help") | None => {
            println!("i2c commands:");
            println!("  i2c scan");
        }

        Some(cmd) => {
            println!("unknown i2c command: {}", cmd);
        }
    }
}

pub(super) static I2C_APP: ShellApp = ShellApp::new(
    "i2c",
    "I2C utils",
    i2c_task,
    I2C_STACK_SIZE,
    Priority(I2C_PRIO),
);
