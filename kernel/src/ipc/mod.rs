mod endpoint;
mod message;
mod message_queue;
mod registry;

use message_queue::MessageQueue;

pub(crate) use endpoint::Endpoint;
pub(crate) use message::Message;
pub use message::{MESSAGE_ARG_COUNT, MessageData};
pub(crate) use registry::{EndpointHandle, EndpointRegistry, IPC_REGISTRY};
