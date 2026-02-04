use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug, Clone)]
pub enum Message {
    /// The choke message is fixed-length and has no payload.
    Choke,

    /// The unchoke message is fixed-length and has no payload.
    Unchoke,

    /// The interested message is fixed-length and has no payload.
    Interested,

    /// The not-interested message is fixed-length and has no payload.
    NotInterested,

    /// The have message is fixed length.
    /// The payload is the zero-based index of a piece that has just been successfully downloaded and verified via the hash.
    Have(u32),

    /// The bitfield message may only be sent immediately after the handshaking sequence is completed, and before any other messages are sent.
    /// It is optional, and need not be sent if a client has no pieces.
    ///
    /// The bitfield message is variable length, where X is the length of the bitfield.
    /// The payload is a bitfield representing the pieces that have been successfully downloaded.
    /// The high bit in the first byte corresponds to piece index 0.
    /// Bits that are cleared indicated a missing piece, and set bits indicate a valid and available piece.
    /// Spare bits at the end are set to zero.
    Bitfield(Vec<u8>),

    ///The request message is fixed length, and is used to request a block. The payload contains the following information:
    ///
    /// index: integer specifying the zero-based piece index
    /// begin: integer specifying the zero-based byte offset within the piece
    /// length: integer specifying the requested length.
    Request { index: u32, begin: u32, length: u32 },

    /// The piece message is variable length, where X is the length of the block. The payload contains the following information:
    ///
    /// index: integer specifying the zero-based piece index
    /// begin: integer specifying the zero-based byte offset within the piece
    /// block: block of data, which is a subset of the piece specified by index.
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },

    /// The cancel message is fixed length, and is used to cancel block requests.
    /// The payload is identical to that of the "request" message.
    /// It is typically used during "End Game"
    Cancel { index: u32, begin: u32, length: u32 },
}

impl Message {
    pub fn serialize(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        let id: u8;

        match self {
            Message::Choke => id = 0,

            Message::Unchoke => id = 1,

            Message::Interested => id = 2,

            Message::NotInterested => id = 3,

            Message::Have(index) => {
                id = 4;
                payload.write_u32::<BigEndian>(*index).unwrap();
            }

            Message::Bitfield(bitfield) => {
                id = 5;
                payload.extend_from_slice(bitfield);
            }

            Message::Request {
                index,
                begin,
                length,
            } => {
                id = 6;
                payload.write_u32::<BigEndian>(*index).unwrap();
                payload.write_u32::<BigEndian>(*begin).unwrap();
                payload.write_u32::<BigEndian>(*length).unwrap();
            }

            Message::Piece {
                index,
                begin,
                block,
            } => {
                id = 7;
                payload.write_u32::<BigEndian>(*index).unwrap();
                payload.write_u32::<BigEndian>(*begin).unwrap();
                payload.extend_from_slice(block);
            }

            Message::Cancel {
                index,
                begin,
                length,
            } => {
                id = 8;
                payload.write_u32::<BigEndian>(*index).unwrap();
                payload.write_u32::<BigEndian>(*begin).unwrap();
                payload.write_u32::<BigEndian>(*length).unwrap();
            }
        }

        // Final structure: [Length (4 bytes)] + [ID (1 byte)] + [Payload]
        // Length = 1 byte for ID + payload length
        let len = (1 + payload.len()) as u32;

        let mut msg = Vec::new();
        msg.write_u32::<BigEndian>(len).unwrap();
        msg.push(id);
        msg.append(&mut payload);

        msg
    }

    pub fn deserialize(id: u8, payload: &[u8]) -> anyhow::Result<Self> {
        let mut rdr = Cursor::new(payload);

        match id {
            0 => Ok(Message::Choke),

            1 => Ok(Message::Unchoke),

            2 => Ok(Message::Interested),

            3 => Ok(Message::NotInterested),

            4 => {
                let index = rdr.read_u32::<BigEndian>()?;
                Ok(Message::Have(index))
            }

            5 => Ok(Message::Bitfield(payload.to_vec())),

            6 => {
                let index = rdr.read_u32::<BigEndian>()?;
                let begin = rdr.read_u32::<BigEndian>()?;
                let length = rdr.read_u32::<BigEndian>()?;
                Ok(Message::Request {
                    index,
                    begin,
                    length,
                })
            }

            7 => {
                let index = rdr.read_u32::<BigEndian>()?;
                let begin = rdr.read_u32::<BigEndian>()?;
                let mut block = Vec::new();
                rdr.read_to_end(&mut block)?;
                Ok(Message::Piece {
                    index,
                    begin,
                    block,
                })
            }

            8 => {
                let index = rdr.read_u32::<BigEndian>()?;
                let begin = rdr.read_u32::<BigEndian>()?;
                let length = rdr.read_u32::<BigEndian>()?;
                Ok(Message::Cancel {
                    index,
                    begin,
                    length,
                })
            }

            _ => anyhow::bail!("Unknown Message ID: {}", id),
        }
    }
}

pub struct Bitfield {
    payload: Vec<u8>,
}

impl Bitfield {
    pub fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }

    pub fn has_piece(&self, index: usize) -> bool {
        let byte_idx = index / 8;
        let byte_offset = index % 8;

        if byte_idx >= self.payload.len() {
            return false;
        }

        (self.payload[byte_idx] >> (7 - byte_offset)) & 1 == 1
    }

    pub fn pieces(&self) -> usize {
        self.payload.len() * 8
    }
}
