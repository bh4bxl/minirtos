mod endpoint;
mod message;
mod message_queue;
mod registry;

use message_queue::MessageQueue;

pub(crate) use endpoint::Endpoint;
pub(crate) use message::Message;
pub(crate) use registry::{EndpointRegistry, IPC_REGISTRY};
