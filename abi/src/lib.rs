#![no_std]

mod error;
mod ipc;
mod syscall;
mod user_ptr;

pub use error::SysError;
pub use ipc::{
    EndpointHandle, IpcRecvArgs, IpcSendArgs, MESSAGE_ARG_COUNT, MessageData, ReceivedMessage,
};
pub use syscall::SyscallId;
pub use user_ptr::{UserMutPtr, UserPtr};
