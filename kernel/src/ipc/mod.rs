mod endpoint;
mod message;
mod message_queue;
mod pending;
mod registry;

use message_queue::MessageQueue;

pub(crate) use endpoint::Endpoint;
pub(crate) use message::{Message, MessagePayload};
pub(crate) use pending::PendingIpc;
pub(crate) use registry::{EndpointRegistry, IPC_REGISTRY};
