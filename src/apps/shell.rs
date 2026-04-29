use crate::{
    print, println,
    sys::{
        console, device_driver, scheduler, syscall,
        task::{Priority, TaskStack},
    },
};

const LINE_LEN: usize = 64;

const SHELL_PRIO: u8 = 100;

const SHELL_STACK_SIZE: usize = 1024;
static SHELL_STACK: TaskStack<SHELL_STACK_SIZE> = TaskStack::new();

pub fn start_shell() -> Result<(), &'static str> {
    if let Err(x) = syscall::thread_create(
        shell_task_entry,
        core::ptr::null_mut(),
        SHELL_STACK.get(),
        Priority(SHELL_PRIO),
        "shell",
    ) {
        Err(x)
    } else {
        Ok(())
    }
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

fn handle_command(cmd: &str) {
    match cmd {
        "" => {}

        "help" => {
            println!("commands:");
            println!("  help      show this message");
            println!("  tick      show system tick");
            println!("  tasks     show task list");
            println!("  devs      show device list");
            println!("  reboot    reset system");
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

        _ => {
            println!("unknown command: {}", cmd);
        }
    }
}
