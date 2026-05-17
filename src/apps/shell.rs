use crate::{
    net::WifiAuth,
    print, println,
    services::wlan_service::FixedStr,
    sys::{
        SysError, console, device_driver, scheduler, syscall,
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

        _ => {
            println!("unknown command: {}", cmd);
        }
    }
}
