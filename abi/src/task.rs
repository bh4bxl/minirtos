#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(usize);

impl TaskId {
    pub const KERNEL: Self = Self(0);

    pub const fn from_raw(id: usize) -> Self {
        Self(id)
    }

    pub fn raw(self) -> usize {
        self.0
    }
}

pub type TaskEntry = extern "C" fn(*mut ());

/// Task priority.
///
/// Lower values indicate higher priority.
///
/// Default priorities:
/// - init task: 10
/// - services: 100
/// - idle task: 255
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub u8);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskCreateArgs {
    pub entry: TaskEntry,
    pub arg: *mut (),
    pub stack_size: usize,
    pub priority: Priority,
    pub name: *const u8,
    pub name_len: usize,
}
