use minirtos_abi::{
    EndpointHandle, IpcCallArgs, IpcCompleteArgs, IpcMessageArgs, IpcOp, IpcRecvArgs,
    ReceivedRequest, SysError, UserPtr,
};

use crate::{
    arch,
    ipc::{EndpointOwner, IPC_REGISTRY, Message, PendingIpc},
    sched,
    service::kernel_service_handle,
    synchronization::{CriticalSection, critical_section},
    task::TaskId,
};

use super::SyscallResult;

mod kernel_service;
mod user;

pub mod endpoint;
pub mod memory;

use kernel_service::kernel_service_call;

pub(crate) use user::{read_user, write_user};

pub(crate) fn ipc_dispatch(op: u32, args: &[u32]) -> SyscallResult {
    let Ok(op) = IpcOp::try_from(op) else {
        return SyscallResult::Error(SysError::NotSupported);
    };

    match op {
        IpcOp::CreateEndpoint => create_endpoint(),

        IpcOp::DestroyEndpoint => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            destroy_endpoint(args[0])
        }

        IpcOp::TrySend => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            try_send(args[0])
        }

        IpcOp::TryRecv => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            try_recv(args[0])
        }

        IpcOp::Send => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            send(args[0])
        }

        IpcOp::Recv => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            recv(args[0])
        }

        IpcOp::Call => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            call(args[0])
        }

        IpcOp::Complete => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            complete(args[0])
        }
    }
}

fn create_endpoint() -> SyscallResult {
    let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let res = critical_section(|cs| {
        IPC_REGISTRY.lock(cs, |registry| registry.create(EndpointOwner::Task(owner)))
    });

    match res {
        Ok(handle) => SyscallResult::U32(handle.raw()),
        Err(err) => SyscallResult::Error(err),
    }
}

fn destroy_endpoint(raw_handle: u32) -> SyscallResult {
    let handle = EndpointHandle::from_raw(raw_handle);

    let task_id = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let owner = EndpointOwner::Task(task_id);

    let res =
        critical_section(|cs| IPC_REGISTRY.lock(cs, |registry| registry.destroy(owner, handle)));

    match res {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn try_send(raw_args: u32) -> SyscallResult {
    let send_args = match read_user(UserPtr::<IpcMessageArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let data = match read_user(send_args.message) {
        Ok(data) => data,
        Err(err) => return SyscallResult::Error(err),
    };

    let sender = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let message = Message::new(sender, data);

    let res = critical_section(|cs| {
        IPC_REGISTRY.lock(cs, |registry| {
            let endpoint = registry.endpoint(send_args.endpoint)?;

            endpoint
                .try_send_cs(cs, message)
                .map_err(|_| SysError::WouldBlock)
        })
    });

    match res {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn try_recv(raw_args: u32) -> SyscallResult {
    let recv_args = match read_user(UserPtr::<IpcRecvArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let result = critical_section(|cs| {
        IPC_REGISTRY.lock(cs, |registry| {
            let endpoint = registry.endpoint(recv_args.endpoint)?;

            endpoint.try_recv_cs(cs).ok_or(SysError::WouldBlock)
        })
    });

    let message = match result {
        Ok(message) => message,
        Err(err) => return SyscallResult::Error(err),
    };

    let request = message_to_request(message);

    match write_user(recv_args.request, request) {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn complete_recv(cs: &CriticalSection, receiver: TaskId, message: Message) -> Result<(), SysError> {
    let sched = sched::scheduler();

    let pending = sched.take_pending_ipc(cs, receiver)?;

    let PendingIpc::Recv { endpoint: _, out } = pending else {
        return Err(SysError::InvalidState);
    };

    let request = message_to_request(message);

    write_user(out, request)?;

    sched.wake_task(cs, receiver);

    Ok(())
}

pub(super) fn send(raw_args: u32) -> SyscallResult {
    let send_args = match read_user(UserPtr::<IpcMessageArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let data = match read_user(send_args.message) {
        Ok(data) => data,
        Err(err) => return SyscallResult::Error(err),
    };

    let result = critical_section(|cs| {
        let sched = sched::scheduler();
        let sender = sched.current_task_id(cs);

        IPC_REGISTRY.lock(cs, |registry| {
            let owner = registry.owner(send_args.endpoint)?;

            if owner == EndpointOwner::KernelService {
                return kernel_service_handle(cs, sender, data).map(|_| ());
            }

            let endpoint = registry.endpoint(send_args.endpoint)?;
            let message = Message::new(sender, data);

            //
            // Direct handoff first.
            //
            if let Some(receiver) = endpoint.pop_receiver_waiter_cs(cs) {
                complete_recv(cs, receiver, message)?;
                return Ok(());
            }

            //
            // Otherwise buffer the message.
            //
            endpoint
                .try_send_cs(cs, message)
                .map_err(|_| SysError::WouldBlock)
        })
    });

    match result {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn recv(raw_args: u32) -> SyscallResult {
    let recv_args = match read_user(UserPtr::<IpcRecvArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let result = critical_section(|cs| {
        IPC_REGISTRY.lock(cs, |registry| {
            let endpoint = registry.endpoint(recv_args.endpoint)?;

            //
            // Message already buffered.
            //
            if let Some(message) = endpoint.try_recv_cs(cs) {
                return Ok(Some(message));
            }

            //
            // Nothing available: block current task.
            //
            let sched = sched::scheduler();
            let tid = sched.current_task_id(cs);

            sched.set_pending_ipc(
                cs,
                tid,
                PendingIpc::Recv {
                    endpoint: recv_args.endpoint,
                    out: recv_args.request,
                },
            )?;

            endpoint.block_receiver_cs(cs);

            Ok(None)
        })
    });

    match result {
        Ok(Some(message)) => {
            let request = message_to_request(message);

            match write_user(recv_args.request, request) {
                Ok(()) => SyscallResult::U32(0),
                Err(err) => SyscallResult::Error(err),
            }
        }

        Ok(None) => {
            //
            // Current task is already Blocked.
            //
            // SVC returns normally and pending PendSV switches away.
            // The sender will later write the output buffer and wake us.
            //
            SyscallResult::U32(0)
        }

        Err(err) => SyscallResult::Error(err),
    }
}

fn call(raw_args: u32) -> SyscallResult {
    let args = match read_user(UserPtr::<IpcCallArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let request = match read_user(args.request) {
        Ok(request) => request,
        Err(err) => return SyscallResult::Error(err),
    };

    let result = critical_section(|cs| {
        let sched = sched::scheduler();
        let caller = sched.current_task_id(cs);

        IPC_REGISTRY.lock(cs, |registry| {
            let owner = registry.owner(args.endpoint)?;

            //
            // KernelService executes synchronously in kernel context.
            //
            if owner == EndpointOwner::KernelService {
                let response = kernel_service_handle(cs, caller, request)?;

                write_user(args.response, response)?;

                return Ok(());
            }

            let endpoint = registry.endpoint(args.endpoint)?;
            let message = Message::new(caller, request);

            //
            // First make sure the request can actually be delivered.
            //
            if let Some(receiver) = endpoint.pop_receiver_waiter_cs(cs) {
                sched.set_pending_ipc(
                    cs,
                    caller,
                    PendingIpc::Call {
                        endpoint: args.endpoint,
                        response: args.response,
                    },
                )?;

                complete_recv(cs, receiver, message)?;
            } else {
                endpoint
                    .try_send_cs(cs, message)
                    .map_err(|_| SysError::WouldBlock)?;

                sched.set_pending_ipc(
                    cs,
                    caller,
                    PendingIpc::Call {
                        endpoint: args.endpoint,
                        response: args.response,
                    },
                )?;
            }

            sched.block_current_task(cs);
            arch::request_context_switch();

            Ok(())
        })
    });

    match result {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn message_to_request(message: Message) -> ReceivedRequest {
    let sender = message.sender();
    let data = message.data();

    ReceivedRequest {
        sender,
        op: data.op,
        args: data.args,
    }
}

fn complete(raw_args: u32) -> SyscallResult {
    let args = match read_user(UserPtr::<IpcCompleteArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let response_message = match read_user(args.response) {
        Ok(response) => response,
        Err(err) => return SyscallResult::Error(err),
    };

    let ret = critical_section(|cs| {
        let sched = sched::scheduler();
        let current = sched.current_task_id(cs);

        // Only the endpoint owner/server may complete requests.
        let owner = IPC_REGISTRY.lock(cs, |registry| registry.owner(args.endpoint))?;

        if owner != EndpointOwner::Task(current) {
            return Err(SysError::InvalidState);
        }

        let pending = sched.take_pending_ipc(cs, args.target)?;

        match pending {
            PendingIpc::Call {
                endpoint: pending_endpoint,
                response,
            } => {
                if pending_endpoint != args.endpoint {
                    return Err(SysError::InvalidState);
                }

                write_user(response, response_message)?;
            }

            _ => {
                return Err(SysError::InvalidState);
            }
        }

        sched.wake_task(cs, args.target);

        Ok(())
    });

    match ret {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}
