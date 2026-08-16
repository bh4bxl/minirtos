use crate::{
    SysError,
    memory::STACK_POOL,
    sched,
    synchronization::{critical_section, interface::Mutex},
};

use super::{Priority, Privilege, TaskEntry, TaskId};

pub struct Task {
    entry: TaskEntry,
    arg: *mut (),
    stack_size: usize,
    priority: Priority,
    privilege: Privilege,
    name: &'static str,
}

const DEFAULT_STACK_SIZE: usize = 2048;
const DEFAULT_PRIORITY: u8 = 128;

impl Task {
    pub const fn new(entry: TaskEntry) -> Self {
        Self {
            entry,
            arg: core::ptr::null_mut(),
            stack_size: DEFAULT_STACK_SIZE,
            priority: Priority(DEFAULT_PRIORITY),
            privilege: Privilege::Unprivileged,
            name: "",
        }
    }

    pub const fn stack_size(mut self, bytes: usize) -> Self {
        self.stack_size = bytes;
        self
    }

    pub const fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub(crate) const fn privilege(mut self, privilege: Privilege) -> Self {
        self.privilege = privilege;
        self
    }

    pub const fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub const fn arg(mut self, arg: *mut ()) -> Self {
        self.arg = arg;
        self
    }

    pub fn spawn(self) -> Result<TaskId, SysError> {
        let stack = STACK_POOL.lock(|pool| pool.alloc(self.stack_size))?;

        match critical_section(|cs| {
            sched::scheduler().add_task(
                cs,
                self.entry,
                self.arg,
                stack,
                self.priority,
                self.privilege,
                self.name,
            )
        }) {
            Ok(id) => Ok(id),

            Err((err, stack)) => {
                STACK_POOL.lock(|pool| {
                    pool.free(stack);
                });

                Err(err)
            }
        }
    }
}
