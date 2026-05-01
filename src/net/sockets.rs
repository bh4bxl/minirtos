use smoltcp::{
    iface::{SocketHandle, SocketSet},
    socket::udp,
};

static mut UDP_RX_META: [udp::PacketMetadata; 8] = [udp::PacketMetadata::EMPTY; 8];

static mut UDP_TX_META: [udp::PacketMetadata; 8] = [udp::PacketMetadata::EMPTY; 8];

static mut UDP_RX_BUF: [u8; 1024] = [0; 1024];
static mut UDP_TX_BUF: [u8; 1024] = [0; 1024];

pub fn create_udp_socket(sockets: &mut SocketSet<'static>) -> SocketHandle {
    let rx_buffer = udp::PacketBuffer::new(unsafe { &mut UDP_RX_META[..] }, unsafe {
        &mut UDP_RX_BUF[..]
    });

    let tx_buffer = udp::PacketBuffer::new(unsafe { &mut UDP_TX_META[..] }, unsafe {
        &mut UDP_TX_BUF[..]
    });

    let socket = udp::Socket::new(rx_buffer, tx_buffer);

    sockets.add(socket)
}

pub fn bind_udp(sockets: &mut SocketSet<'static>, handle: SocketHandle, port: u16) {
    let socket = sockets.get_mut::<udp::Socket>(handle);

    socket.bind(port).unwrap();
}
