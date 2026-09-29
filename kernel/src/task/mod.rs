mod control;
mod task;

pub(crate) use control::TaskControl;
use minirtos_abi::Priority;
pub use task::Task;

pub(crate) use minirtos_abi::TaskId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Privilege {
    Privileged,
    Unprivileged,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Sleeping,
    Suspended,
    Terminated,
}

impl TaskState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Running => "Running",
            Self::Blocked => "Blocked",
            Self::Sleeping => "Sleep",
            Self::Suspended => "Suspended",
            Self::Terminated => "Terminated",
        }
    }
}

pub type TaskExit = extern "C" fn() -> !;

#[derive(Clone)]
pub struct TaskInfo {
    pub id: TaskId,
    pub name: &'static str,
    pub state: TaskState,
    pub priority: Priority,
    pub(crate) privilege: Privilege,
    pub stack_used: usize,
    pub stack_total: usize,
}
