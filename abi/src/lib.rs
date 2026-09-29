#![no_std]

mod error;
mod ipc;
mod memory;
mod service;
mod syscall;
mod task;
mod user_ptr;

pub use error::SysError;
pub use ipc::{
    EndpointHandle, IpcCallArgs, IpcCompleteArgs, IpcMessageArgs, IpcOp, IpcRecvArgs,
    MESSAGE_ARG_COUNT, MessageData, ReceivedRequest,
};
pub use memory::{
    Aligned32, SharedBufferHandle, SharedBufferInfo, SharedBufferRef, align_down, align_up,
};
pub use service::{ServiceId, ServiceOp};
pub use syscall::SyscallId;
pub use task::{Priority, TaskCreateArgs, TaskEntry, TaskId};
pub use user_ptr::{UserMutPtr, UserPtr};
