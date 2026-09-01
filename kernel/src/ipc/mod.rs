mod endpoint;
mod message;
mod message_queue;
mod pending;
mod registry;
mod shared_buffer;

use message_queue::MessageQueue;

pub(crate) use endpoint::Endpoint;
pub(crate) use message::{Message, MessagePayload};
pub(crate) use pending::PendingIpc;
pub(crate) use registry::{EndpointOwner, IPC_REGISTRY};
pub(crate) use shared_buffer::{
    shared_buffer_create, shared_buffer_destroy, shared_buffer_map, shared_buffer_unmap,
};
