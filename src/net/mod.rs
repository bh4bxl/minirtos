#![cfg(feature = "cyw43")]
#![allow(dead_code)]

pub mod core;
pub mod service;

mod buffer;
mod error;
mod io;
mod request;
mod request_api;
mod socket;
mod tcp;

use buffer::{BufferId, NetBuffer};
use buffer::{NET_BUFFER_SIZE, with_buffer, with_buffer_mut};
pub use error::{NetError, NetResult};
pub use io::{Read, Write};
use request::{NetResponse, RequestId, complete_request};
use socket::SocketId;
pub use tcp::TcpStream;
