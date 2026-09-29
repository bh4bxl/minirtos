use minirtos_abi::{EndpointHandle, MessageData, ReceivedRequest, UserMutPtr};

#[derive(Clone, Copy)]
pub(crate) enum PendingIpc {
    Recv {
        endpoint: EndpointHandle,
        out: UserMutPtr<ReceivedRequest>,
    },

    Call {
        endpoint: EndpointHandle,
        response: UserMutPtr<MessageData>,
    },
}
