use std::net::SocketAddrV4;

#[derive(Debug)]
pub struct Peer {
    pub addr: SocketAddrV4
}

impl Peer {
    pub fn new(addr: SocketAddrV4) -> Self {
        Self { addr }
    }
}
