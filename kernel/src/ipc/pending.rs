use minirtos_abi::{EndpointHandle, ReceivedRequest, UserMutPtr, UserPtr};

#[derive(Clone, Copy)]
pub(crate) enum PendingIpc {
    None,

    Recv {
        endpoint: EndpointHandle,
        out: UserMutPtr<ReceivedRequest>,
    },

    Write {
        endpoint: EndpointHandle,
        op: u32,
        ptr: UserPtr<u8>,
        len: usize,
    },

    Read {
        endpoint: EndpointHandle,
        op: u32,
        ptr: UserMutPtr<u8>,
        len: usize,
    },
}
