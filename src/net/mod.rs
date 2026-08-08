#![cfg(feature = "cyw43")]
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod core;
pub mod service;

mod buffer;
mod dns;
mod error;
mod icmp;
mod io;
mod request;
mod request_api;
mod socket;
mod state;
mod tcp;

use buffer::{BufferId, NetBuffer};
use buffer::{NET_BUFFER_SIZE, with_buffer, with_buffer_mut};
pub use dns::{resolve, resolve_timeout};
pub use error::{NetError, NetResult};
pub use icmp::{PingReply, ping, ping_timeout};
pub use io::{Read, Write};
use request::{NetResponse, RequestId, complete_request};
use socket::SocketId;
pub use state::{Ipv4Config, NetworkStatus, network_config, network_status, wait_network};
pub(crate) use state::{set_network_config, set_network_configuring, set_network_down};
pub use tcp::TcpStream;
