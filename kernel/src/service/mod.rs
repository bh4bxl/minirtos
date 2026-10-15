mod kernel_service;
mod registry;

pub(crate) use kernel_service::{
    InterruptOp, KernelServiceClass, KernelServiceResult, MemoryOp, kernel_service,
    kernel_service_handle, make_op, split_op,
};
pub(crate) use registry::{SERVICE_REGISTRY, ServiceRegistry};
