use minirtos_abi::{
    EndpointHandle, IpcOp, IpcRecvArgs, IpcSendArgs, ReceivedMessage, SysError, UserPtr,
};

use crate::{
    ipc::{IPC_REGISTRY, Message},
    sched,
    synchronization::{CriticalSection, critical_section},
    task::{PendingIpc, TaskId},
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

    let received = ReceivedMessage {
        sender: message.sender().raw() as u32,
        data: message.data(),
    };

    match write_user(recv_args.message, received) {
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

    let received = ReceivedMessage {
        sender: message.sender().raw() as u32,
        data: message.data(),
    };

    write_user(out, received)?;

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

    let message = Message::new(sender, data);

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
                    out: recv_args.message,
                },
            )?;

            endpoint.block_receiver_cs(cs);

            Ok(None)
        })
    });

    match result {
        Ok(Some(message)) => {
            let received = ReceivedMessage {
                sender: message.sender().raw() as u32,
                data: message.data(),
            };

            match write_user(recv_args.message, received) {
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
