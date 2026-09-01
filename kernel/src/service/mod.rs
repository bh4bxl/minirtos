mod kernel_service;
mod registry;

pub(crate) use kernel_service::{
    KernelServiceClass, MemoryOp, kernel_service, kernel_service_handle, kernel_service_read,
    make_op, split_op,
};
pub(crate) use registry::{SERVICE_REGISTRY, ServiceRegistry};
