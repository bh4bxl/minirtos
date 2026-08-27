use minirtos_abi::{
    EndpointHandle, IpcMessageKind, IpcOp, IpcReadArgs, IpcRecvArgs, IpcSendArgs, IpcWriteArgs,
    MESSAGE_ARG_COUNT, ReceivedRequest, SysError, UserPtr,
};

use crate::{
    arch,
    ipc::{IPC_REGISTRY, Message, MessagePayload, PendingIpc},
    sched,
    synchronization::{CriticalSection, critical_section},
    task::TaskId,
};

use super::SyscallResult;

mod user;

pub mod endpoint;

use user::{read_user, write_user};

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

        IpcOp::Write => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            write(args[0])
        }

        IpcOp::Read => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            read(args[0])
        }

        IpcOp::Complete => {
            if args.is_empty() {
                return SyscallResult::Error(SysError::InvalidArgument);
            }

            complete(args)
        }
    }
}

fn create_endpoint() -> SyscallResult {
    let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let res = critical_section(|cs| IPC_REGISTRY.lock(cs, |registry| registry.create(owner)));

    match res {
        Ok(handle) => SyscallResult::U32(handle.raw()),
        Err(err) => SyscallResult::Error(err),
    }
}

fn destroy_endpoint(raw_handle: u32) -> SyscallResult {
    let handle = EndpointHandle::from_raw(raw_handle);

    let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let res =
        critical_section(|cs| IPC_REGISTRY.lock(cs, |registry| registry.destroy(owner, handle)));

    match res {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn try_send(raw_args: u32) -> SyscallResult {
    let send_args = match read_user(UserPtr::<IpcSendArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let data = match read_user(send_args.message) {
        Ok(data) => data,
        Err(err) => return SyscallResult::Error(err),
    };

    let sender = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let message = Message::data(sender, data);

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

fn send(raw_args: u32) -> SyscallResult {
    let send_args = match read_user(UserPtr::<IpcSendArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    let data = match read_user(send_args.message) {
        Ok(data) => data,
        Err(err) => return SyscallResult::Error(err),
    };

    let sender = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let message = Message::data(sender, data);

    let result = critical_section(|cs| {
        IPC_REGISTRY.lock(cs, |registry| {
            let endpoint = registry.endpoint(send_args.endpoint)?;

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

fn write(raw_args: u32) -> SyscallResult {
    let args = match read_user(UserPtr::<IpcWriteArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    if args.ptr.is_null() && args.len != 0 {
        return SyscallResult::Error(SysError::InvalidArgument);
    }

    let result = critical_section(|cs| {
        let sched = sched::scheduler();
        let sender = sched.current_task_id(cs);

        let message = Message::write(sender, args.op, args.ptr, args.len);

        IPC_REGISTRY.lock(cs, |registry| {
            let endpoint = registry.endpoint(args.endpoint)?;

            sched.set_pending_ipc(
                cs,
                sender,
                PendingIpc::Write {
                    endpoint: args.endpoint,
                    op: args.op,
                    ptr: args.ptr,
                    len: args.len,
                },
            )?;

            if let Some(receiver) = endpoint.pop_receiver_waiter_cs(cs) {
                complete_recv(cs, receiver, message)?;
            } else {
                endpoint
                    .try_send_cs(cs, message)
                    .map_err(|_| SysError::WouldBlock)?;
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

fn read(raw_args: u32) -> SyscallResult {
    let args = match read_user(UserPtr::<IpcReadArgs>::from_raw(raw_args)) {
        Ok(args) => args,
        Err(err) => return SyscallResult::Error(err),
    };

    if args.ptr.is_null() && args.len != 0 {
        return SyscallResult::Error(SysError::InvalidArgument);
    }

    let result = critical_section(|cs| {
        let sched = sched::scheduler();
        let sender = sched.current_task_id(cs);

        let message = Message::read(sender, args.op, args.ptr, args.len);

        IPC_REGISTRY.lock(cs, |registry| {
            let endpoint = registry.endpoint(args.endpoint)?;

            sched.set_pending_ipc(
                cs,
                sender,
                PendingIpc::Read {
                    endpoint: args.endpoint,
                    op: args.op,
                    ptr: args.ptr,
                    len: args.len,
                },
            )?;

            if let Some(receiver) = endpoint.pop_receiver_waiter_cs(cs) {
                complete_recv(cs, receiver, message)?;
            } else {
                endpoint
                    .try_send_cs(cs, message)
                    .map_err(|_| SysError::WouldBlock)?;
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

    match message.payload() {
        MessagePayload::Data(data) => ReceivedRequest {
            sender,
            kind: IpcMessageKind::Data,
            op: data.op,
            args: data.args,
            ptr: 0,
            len: 0,
        },

        MessagePayload::Write { op, ptr, len } => ReceivedRequest {
            sender,
            kind: IpcMessageKind::Write,
            op,
            args: [0; MESSAGE_ARG_COUNT],
            ptr: ptr.raw(),
            len,
        },

        MessagePayload::Read { op, ptr, len } => ReceivedRequest {
            sender,
            kind: IpcMessageKind::Read,
            op,
            args: [0; MESSAGE_ARG_COUNT],
            ptr: ptr.raw(),
            len,
        },
    }
}

fn complete(args: &[u32]) -> SyscallResult {
    if args.len() < 3 {
        return SyscallResult::Error(SysError::InvalidArgument);
    }

    let endpoint = EndpointHandle::from_raw(args[0]);
    let target = TaskId::from_raw(args[1] as usize);
    let result = args[2] as i32;

    let current = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let ret = critical_section(|cs| {
        // Only the endpoint owner/server may complete requests.
        let owner = IPC_REGISTRY.lock(cs, |registry| registry.owner(endpoint))?;

        if owner != current {
            return Err(SysError::InvalidState);
        }

        let sched = sched::scheduler();

        let pending = sched.take_pending_ipc(cs, target)?;

        match pending {
            PendingIpc::Write {
                endpoint: pending_endpoint,
                ..
            }
            | PendingIpc::Read {
                endpoint: pending_endpoint,
                ..
            } => {
                if pending_endpoint != endpoint {
                    return Err(SysError::InvalidState);
                }
            }

            _ => {
                return Err(SysError::InvalidState);
            }
        }

        sched.set_syscall_result(cs, target, result)?;
        sched.wake_task(cs, target);

        Ok(())
    });

    match ret {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}
