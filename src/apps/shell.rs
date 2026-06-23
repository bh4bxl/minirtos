use crate::{
    gui::{self, input},
    net::WifiAuth,
    print, println,
    services::wlan_service::FixedStr,
    sys::{
        SysError, console, device_driver, scheduler,
        syscall::{self, sleep_ms},
        task::{Priority, Task},
    },
};

use super::wlan::*;

const LINE_LEN: usize = 64;

const SHELL_PRIO: u8 = 100;

const SHELL_STACK_SIZE: usize = 1024;

pub fn start_shell() -> Result<(), SysError> {
    let mut shell = Task::<SHELL_STACK_SIZE>::new(shell_task_entry)
        .priority(Priority(SHELL_PRIO))
        .name("shell");

    shell.run()?;

    Ok(())
}

/// Thread entry
extern "C" fn shell_task_entry(_arg: *mut ()) -> ! {
    println!("\r\nminiRTOS shell");
    println!("type 'help' for commands");

    loop {
        print!("minitos> ");
        let line = console::read_line::<LINE_LEN>();
        handle_command(line.trim());
    }
}

fn handle_wifi_command<'a>(argv: &mut impl Iterator<Item = &'a str>) {
    match argv.next() {
        Some("scan") => {
            println!("wifi scanning...");

            WLAN_CMD_QUEUE.send(WlanCmd::Scan);

            WLAN_SCAN_DONE.wait();

            println!("wifi scan done");
        }

        Some("connect") => {
            let ssid = match argv.next() {
                Some(v) => v,
                None => {
                    println!("missing ssid");
                    return;
                }
            };

            let password = argv.next();

            let (password, auth) = match password {
                Some(pw) => (Some(FixedStr::from_str(pw).unwrap()), WifiAuth::Wpa2AesPsk),

                None => (None, WifiAuth::Open),
            };

            println!("connecting to {}", ssid);

            WLAN_CMD_QUEUE.send(WlanCmd::Connect {
                ssid: FixedStr::from_str(ssid).unwrap(),
                password,
                auth,
            });

            WLAN_CONNECT_DONE.wait();

            println!("wifi connect done");
        }

        Some("disconnect") => {
            println!("disconnecting");

            WLAN_CMD_QUEUE.send(WlanCmd::Disconnect);

            WLAN_DISCONNECT_DONE.wait();

            println!("wifi disconnect done");
        }

        Some("help") | None => {
            println!("wifi commands:");
            println!("  wifi scan");
            println!("  wifi connect <ssid> <password>");
            println!("  wifi disconnect");
        }

        Some(cmd) => {
            println!("unknown wifi command: {}", cmd);
        }
    }
}

fn handle_i2c_command<'a>(argv: &mut impl Iterator<Item = &'a str>) {
    match argv.next() {
        Some("scan") => {
            println!("i2c scan:");

            let Some(dev) =
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
            println!("unknown wifi command: {}", cmd);
        }
    }
}

fn handle_touch_test() {
    println!("touch test started, press q to quit");

    input::input_manager::InputManager::pause(true);
    loop {
        match gui::input::touch().read_point() {
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

fn handle_kbd_test() {
    println!("keyboard test started, press q to quit");

    input::input_manager::InputManager::pause(true);
    loop {
        match gui::input::keyboard().read_key_value() {
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

fn handle_command(cmd_line: &str) {
    let mut argv = cmd_line.split_ascii_whitespace();

    let Some(cmd) = argv.next() else {
        return;
    };

    match cmd {
        "help" => {
            println!("commands:");
            println!("  help      show this message");
            println!("  tick      show system tick");
            println!("  tasks     show task list");
            println!("  devs      show device list");
            println!("  reboot    reset system");
            println!("  wifi      Wi-Fi commands");
            println!("  ping      ping gateway");
            println!("  i2c       i2c commands");
            println!("  touch     touch test");
            println!("  kbd       keyboard test");
        }

        "tick" => {
            let tick = syscall::get_tick();
            println!("tick={}", tick);
        }

        "tasks" => {
            scheduler::scheduler().dump_tasks();
        }

        "devs" => {
            device_driver::driver_manager().dump_device();
        }

        "reboot" => {
            println!("rebooting...");
            cortex_m::peripheral::SCB::sys_reset();
        }

        "wifi" => {
            handle_wifi_command(&mut argv);
        }

        "ping" => {
            WLAN_CMD_QUEUE.send(WlanCmd::Ping);
        }

        "i2c" => {
            handle_i2c_command(&mut argv);
        }

        "touch" => {
            handle_touch_test();
        }

        "kbd" => {
            handle_kbd_test();
        }

        _ => {
            println!("unknown command: {}", cmd);
        }
    }
}
