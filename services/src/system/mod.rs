use minirtos_abi::{Priority, SysError, TaskId};
use minirtos_kernel::{interface::service::Service, task::Task};

pub mod console;

pub struct ServiceConfig {
    pub name: &'static str,
    pub stack_size: usize,
    pub priority: Priority,
}

pub fn start_system_service<S>(
    service: &'static mut S,
    config: &ServiceConfig,
) -> Result<TaskId, SysError>
where
    S: Service + 'static,
{
    Task::new(service_entry::<S>)
        .arg(service as *mut S as *mut ())
        .stack_size(config.stack_size)
        .priority(config.priority)
        .name(config.name)
        .spawn()
}

extern "C" fn service_entry<S: Service>(arg: *mut ()) {
    let service = unsafe { &mut *(arg as *mut S) };

    service.run();
}
