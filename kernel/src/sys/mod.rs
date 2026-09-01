use crate::SysError;

mod io;
mod ipc;
mod service;
mod sync;
mod task;

pub use io::interface::{Read, Write};
pub use ipc::endpoint::Endpoint;
pub use ipc::memory::SharedBuffer;
pub(crate) use ipc::{ipc_dispatch, read_user, write_user};
pub use service::service::Service;
pub(crate) use service::service_dispatch;
pub use sync::event::Event;
pub use sync::mutex::Mutex;
pub use sync::semaphore::Semaphore;
pub(crate) use sync::sync_dispatch;
pub(crate) use task::task_dispatch;
pub use task::{exit, get_tick, sleep_ms, yield_now};

pub(crate) use minirtos_abi::SyscallId;

pub(crate) enum SyscallResult {
    None,
    U32(u32),
    U64(u64),
    Error(SysError),
}

#[inline]
fn syscall_result(ret: u32) -> Result<u32, SysError> {
    let value = ret as i32;

    if value >= 0 {
        return Ok(ret);
    }

    match SysError::try_from(value) {
        Ok(err) => Err(err),
        Err(_) => Err(SysError::ProtocolError),
    }
}
