use minirtos_abi::{IpcCallArgs, IpcOp, MessageData, SysError, SyscallId, UserMutPtr, UserPtr};

use crate::{arch::syscall, service::kernel_service, synchronization::critical_section};

use super::super::syscall_result;

pub(super) fn kernel_service_call(request: &MessageData) -> Result<MessageData, SysError> {
    let endpoint = critical_section(|cs| kernel_service().lock(cs, |service| service.endpoint()))?;

    let mut response = MessageData::default();

    let args = IpcCallArgs {
        endpoint,
        request: UserPtr::from_raw(request as *const MessageData as u32),
        response: UserMutPtr::from_raw(&mut response as *mut MessageData as u32),
    };

    let result = syscall::<{ SyscallId::Ipc as u8 }>(&[
        IpcOp::Call as u32,
        &args as *const IpcCallArgs as u32,
    ]);

    syscall_result(result)?;

    Ok(response)
}
