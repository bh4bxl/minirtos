#![no_std]

mod error;
mod ipc;
mod service;
mod syscall;
mod task;
mod user_ptr;

pub use error::SysError;
pub use ipc::{
    EndpointHandle, IpcMessageKind, IpcOp, IpcReadArgs, IpcRecvArgs, IpcSendArgs, IpcWriteArgs,
    MESSAGE_ARG_COUNT, MessageData, ReceivedRequest,
};
pub use service::{ServiceId, ServiceOp};
pub use syscall::SyscallId;
pub use task::TaskId;
pub use user_ptr::{UserMutPtr, UserPtr};
