use minirtos_abi::{EndpointHandle, ServiceId, ServiceOp, SysError};

use crate::{
    ipc::IPC_REGISTRY, sched, service::SERVICE_REGISTRY, synchronization::critical_section,
};

use super::SyscallResult;

pub mod service;

pub(crate) fn service_dispatch(op: u32, args: &[u32]) -> SyscallResult {
    let Ok(op) = ServiceOp::try_from(op) else {
        return SyscallResult::Error(SysError::NotSupported);
    };

    match op {
        ServiceOp::Register => register(args),
        ServiceOp::Lookup => lookup(args),
        ServiceOp::Unregister => unregister(args),
    }
}

fn register(args: &[u32]) -> SyscallResult {
    if args.len() < 2 {
        return SyscallResult::Error(SysError::InvalidArgument);
    }

    let id = ServiceId::from_raw(args[0]);
    let endpoint = EndpointHandle::from_raw(args[1]);

    let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let endpoint_owner =
        critical_section(|cs| IPC_REGISTRY.lock(cs, |registry| registry.owner(endpoint)));

    let endpoint_owner = match endpoint_owner {
        Ok(owner) => owner,
        Err(err) => return SyscallResult::Error(err),
    };

    if endpoint_owner != owner {
        return SyscallResult::Error(SysError::InvalidState);
    }

    let result = critical_section(|cs| {
        SERVICE_REGISTRY.lock(cs, |registry| registry.register(owner, id, endpoint))
    });

    match result {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}

fn lookup(args: &[u32]) -> SyscallResult {
    if args.is_empty() {
        return SyscallResult::Error(SysError::InvalidArgument);
    }

    let id = ServiceId::from_raw(args[0]);

    let result = critical_section(|cs| SERVICE_REGISTRY.lock(cs, |registry| registry.lookup(id)));

    match result {
        Ok(endpoint) => SyscallResult::U32(endpoint.raw()),
        Err(err) => SyscallResult::Error(err),
    }
}

fn unregister(args: &[u32]) -> SyscallResult {
    if args.is_empty() {
        return SyscallResult::Error(SysError::InvalidArgument);
    }

    let id = ServiceId::from_raw(args[0]);

    let owner = critical_section(|cs| sched::scheduler().current_task_id(cs));

    let result =
        critical_section(|cs| SERVICE_REGISTRY.lock(cs, |registry| registry.unregister(owner, id)));

    match result {
        Ok(()) => SyscallResult::U32(0),
        Err(err) => SyscallResult::Error(err),
    }
}
