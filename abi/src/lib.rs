#![no_std]

mod error;
mod ipc;
mod service;
mod syscall;
mod user_ptr;

pub use error::SysError;
pub use ipc::{
    EndpointHandle, IpcOp, IpcRecvArgs, IpcSendArgs, MESSAGE_ARG_COUNT, MessageData,
    ReceivedMessage,
};
pub use service::{ServiceId, ServiceOp};
pub use syscall::SyscallId;
pub use user_ptr::{UserMutPtr, UserPtr};
