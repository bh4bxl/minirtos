use alloc::{boxed::Box, vec::Vec};

use minirtos_abi::SysError;
use minirtos_kernel::task::{Priority, Task};

pub trait DriverService {
    fn run(&mut self) -> !;
}

pub struct DriverServiceConfig {
    pub name: &'static str,
    pub stack_size: usize,
    pub priority: Priority,
}

struct DriverServiceEntry {
    config: DriverServiceConfig,
    service: Box<dyn DriverService>,
}

struct DriverServiceContext {
    service: Box<dyn DriverService>,
}

pub struct DriverServiceTable {
    services: Vec<DriverServiceEntry>,
}

impl DriverServiceTable {
    pub const fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    pub fn register<S>(&mut self, config: DriverServiceConfig, service: S) -> Result<(), SysError>
    where
        S: DriverService + 'static,
    {
        self.services
            .try_reserve(1)
            .map_err(|_| SysError::NoMemory)?;

        self.services.push(DriverServiceEntry {
            config,
            service: Box::new(service),
        });

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.services.len()
    }

    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    pub fn spawn_all(self) -> Result<(), SysError> {
        for entry in self.services {
            let DriverServiceEntry { config, service } = entry;

            let context = Box::leak(Box::new(DriverServiceContext { service }));

            Task::new(driver_service_entry)
                .arg(context as *mut DriverServiceContext as *mut ())
                .stack_size(config.stack_size)
                .priority(config.priority)
                .name(config.name)
                .spawn()?;
        }

        Ok(())
    }
}

extern "C" fn driver_service_entry(arg: *mut ()) {
    let context = unsafe { &mut *(arg as *mut DriverServiceContext) };

    context.service.run();
}
