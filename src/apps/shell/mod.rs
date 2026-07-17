use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    print, println,
    sys::{
        SysError, console,
        synchronization::{CriticalSectionLock, critical_section},
        syscall,
        task::{Priority, Task, TaskEntry},
    },
};

mod device;
mod i2c;
mod keyboard;
mod memory;
mod ping;
mod task;
mod touch;
mod wifi;

pub(super) struct AppContext {
    argv: Vec<String>,
}

#[allow(dead_code)]
impl AppContext {
    pub fn new<'a, I>(args: I) -> Self
    where
        I: Iterator<Item = &'a str>,
    {
        Self {
            argv: args.map(ToString::to_string).collect(),
        }
    }

    pub fn argc(&self) -> usize {
        self.argv.len()
    }

    pub fn arg(&self, index: usize) -> Option<&str> {
        self.argv.get(index).map(String::as_str)
    }

    pub fn args(&self) -> impl Iterator<Item = &str> {
        self.argv.iter().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.argv.is_empty()
    }
}

pub(super) struct ShellApp {
    name: &'static str,
    help: &'static str,
    entry: TaskEntry,
    stack_words: usize,
    priority: Priority,
}

impl ShellApp {
    const fn new(
        name: &'static str,
        help: &'static str,
        entry: TaskEntry,
        stack_words: usize,
        priority: Priority,
    ) -> Self {
        Self {
            name,
            help,
            entry,
            stack_words,
            priority,
        }
    }

    fn run(&self, context: AppContext) -> Result<(), SysError> {
        let context = Box::new(context);
        let arg = Box::into_raw(context).cast::<()>();

        let task_id = match syscall::task_spawn(
            self.entry,
            arg,
            self.stack_words,
            self.priority,
            self.name,
        ) {
            Ok(task_id) => task_id,

            Err(error) => {
                unsafe {
                    drop(Box::from_raw(arg.cast::<AppContext>()));
                }

                return Err(error);
            }
        };

        syscall::task_wait(task_id)
    }
}

pub(super) unsafe fn take_context(arg: *mut ()) -> Box<AppContext> {
    assert!(!arg.is_null(), "app context is null");

    unsafe { Box::from_raw(arg.cast::<AppContext>()) }
}

struct AppManagerInner {
    apps: Vec<&'static ShellApp>,
}

impl AppManagerInner {
    const fn new() -> Self {
        Self { apps: Vec::new() }
    }
}

pub(super) struct AppManager {
    inner: CriticalSectionLock<AppManagerInner>,
}

impl AppManager {
    pub const fn new() -> Self {
        Self {
            inner: CriticalSectionLock::new(AppManagerInner::new()),
        }
    }

    pub fn register_app(&self, app: &'static ShellApp) -> Result<(), SysError> {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                if inner.apps.iter().any(|a| a.name == app.name) {
                    return Err(SysError::AlreadyExists);
                }

                inner.apps.push(app);
                Ok(())
            })
        })
    }

    fn find(&self, name: &str) -> Option<&'static ShellApp> {
        critical_section(|cs| {
            self.inner.lock(cs, |inner| {
                inner.apps.iter().copied().find(|app| app.name == name)
            })
        })
    }

    fn enumerate(&self) -> Vec<&'static ShellApp> {
        critical_section(|cs| self.inner.lock(cs, |inner| inner.apps.clone()))
    }

    fn show_help(&self) -> Result<(), SysError> {
        println!("Available commands:");

        println!();
        println!("  {:<16}{}", "help", "Show this message");
        println!("  {:<16}{}", "reboot", "Reset system");
        println!("  {:<16}{}", "tick", "Show system tick");
        for app in self.enumerate() {
            println!("  {:<16}{}", app.name, app.help);
        }
        println!();

        Ok(())
    }

    pub fn run(&self, cmd_line: &str) -> Result<(), SysError> {
        let mut argv = cmd_line.split_ascii_whitespace();

        let Some(cmd) = argv.next() else {
            return Ok(());
        };

        match cmd {
            "help" => self.show_help(),
            "reboot" => {
                println!("rebooting...");
                cortex_m::peripheral::SCB::sys_reset();
            }
            "tick" => {
                let tick = syscall::get_tick();
                println!("tick={}", tick);
                Ok(())
            }
            _ => {
                let app = self.find(cmd).ok_or(SysError::NotFound)?;
                let context = AppContext::new(argv);

                app.run(context)
            }
        }
    }
}

static APP_MANAGER: AppManager = AppManager::new();

pub(super) fn app_manager() -> &'static AppManager {
    &APP_MANAGER
}

const LINE_LEN: usize = 64;

const SHELL_PRIO: u8 = 100;
const SHELL_STACK_SIZE: usize = 512;

pub fn start_shell() -> Result<(), SysError> {
    let mut shell = Task::<SHELL_STACK_SIZE>::new(shell_task_entry)
        .priority(Priority(SHELL_PRIO))
        .name("shell");

    shell.run()?;

    Ok(())
}

/// Thread entry
extern "C" fn shell_task_entry(_arg: *mut ()) {
    println!("\r\nminiRTOS shell");
    println!("type 'help' for commands");

    app_manager().register_app(&device::DEVS_APP).ok();
    app_manager().register_app(&memory::MEM_APP).ok();
    app_manager().register_app(&task::TASKS_APP).ok();
    app_manager().register_app(&i2c::I2C_APP).ok();
    app_manager().register_app(&touch::TOUCH_APP).ok();
    app_manager().register_app(&keyboard::KEYBOARD_APP).ok();
    #[cfg(feature = "cyw43")]
    app_manager().register_app(&wifi::WIFI_APP).ok();
    #[cfg(feature = "cyw43")]
    app_manager().register_app(&ping::PING_APP).ok();

    loop {
        print!("minitos> ");
        let cmd = console::read_line::<LINE_LEN>();
        //handle_command(line.trim());
        match app_manager().run(cmd.trim()) {
            Ok(()) => {}
            Err(SysError::NotFound) => println!("unknown command '{}'", cmd),
            Err(e) => println!("failed to run '{}': {:?}", cmd, e),
        }
    }
}
