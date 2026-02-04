use std::net::SocketAddrV4;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct Peer {
    pub addr: SocketAddrV4,
    pub stream: Option<tokio::net::TcpStream>,
}

impl Peer {
    pub fn new(addr: SocketAddrV4) -> Self {
        Self { addr, stream: None }
    }

    pub async fn handshake(
        &mut self,
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> anyhow::Result<[u8; 20]> {
        let mut stream = tokio::net::TcpStream::connect(self.addr)
            .await
            .context("Failed to connect to peer")?;

        let handshake = Handshake::new(info_hash, peer_id);
        let handshake_bytes = handshake.as_bytes();

        stream
            .write_all(&handshake_bytes)
            .await
            .context("Failed to write handshake")?;

        let mut buf = [0u8; 68];
        stream
            .read_exact(&mut buf)
            .await
            .context("Failed to read handshake")?;

        let response = Handshake::from_bytes(&buf).context("Failed to parse handshake")?;

        if response.info_hash != info_hash {
            anyhow::bail!("Info hash mismatch");
        }

        self.stream = Some(stream);

        Ok(response.peer_id)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Handshake {
    /// string length of <pstr>, as a single raw byte
    pstrlen: u8,

    /// string identifier of the protocol
    pstr: [u8; 19],

    /// eight (8) reserved bytes. All current implementations use all zeroes.
    /// Each bit in these bytes can be used to change the behavior of the protocol.
    /// An email from Bram suggests that trailing bits should be used first, so that leading bits may be used to change the meaning of trailing bits.
    reserved: [u8; 8],

    /// 20-byte SHA1 hash of the info key in the metainfo file.
    /// This is the same info_hash that is transmitted in tracker requests.
    info_hash: [u8; 20],

    /// 20-byte string used as a unique ID for the client.
    /// This is usually the same peer_id that is transmitted in tracker requests (but not always e.g. an anonymity option in Azureus).
    peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            pstrlen: 19,
            pstr: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(68);

        bytes.push(self.pstrlen);
        bytes.extend_from_slice(&self.pstr);
        bytes.extend_from_slice(&self.reserved);
        bytes.extend_from_slice(&self.info_hash);
        bytes.extend_from_slice(&self.peer_id);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes[0] != 19 {
            anyhow::bail!("Invalid protocol length: {}", bytes[0]);
        }

        if &bytes[1..20] != b"BitTorrent protocol" {
            anyhow::bail!(
                "Invalid protocol string: {}",
                String::from_utf8_lossy(&bytes[1..20])
            );
        }

        let mut pstr = [0u8; 19];
        pstr.copy_from_slice(&bytes[1..20]);

        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&bytes[20..28]);

        let mut info_hash = [0u8; 20];
        info_hash.copy_from_slice(&bytes[28..48]);

        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&bytes[48..68]);

        Ok(Self {
            pstrlen: bytes[0],
            pstr,
            reserved,
            info_hash,
            peer_id,
        })
    }
}
