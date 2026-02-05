use std::io::SeekFrom;

use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::{
    constants::{self, BLOCK_MAX},
    message::{Bitfield, Message},
    peer::Peer,
    queue::WorkQueue,
};

pub struct DownloadWorker {
    peer: Peer,
    queue: std::sync::Arc<WorkQueue>,
    file: std::sync::Arc<tokio::sync::Mutex<tokio::fs::File>>,
    am_interested: bool,
    peer_choking: bool,
    bitfield: Option<Bitfield>,
    piece_length: u32,
    file_length: u64,
}

impl DownloadWorker {
    pub fn new(
        peer: Peer,
        queue: std::sync::Arc<WorkQueue>,
        file: std::sync::Arc<tokio::sync::Mutex<tokio::fs::File>>,
        piece_length: u32,
        file_length: u64,
    ) -> Self {
        Self {
            peer,
            queue,
            file,
            am_interested: false,
            peer_choking: true,
            bitfield: None,
            piece_length,
            file_length,
        }
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        println!("Work started for peer {}", self.peer.addr);

        loop {
            let msg = self.peer.next_message().await?;

            match msg {
                Message::Bitfield(payload) => {
                    println!("Received bitfield");
                    self.bitfield = Some(Bitfield::new(payload));

                    self.send_interested().await?;
                }

                Message::Unchoke => {
                    println!("Unchoked, receiving data");
                    self.peer_choking = false;

                    match self.request_loop().await {
                        Ok(_) => {
                            println!("Download Complete");
                            return Ok(());
                        }
                        Err(e) => {
                            println!("Download Failed: {}", e);
                            return Err(e);
                        }
                    }
                }

                Message::Choke => {
                    self.peer_choking = true;
                    println!("Choked");
                }

                Message::Piece {
                    index,
                    begin,
                    block,
                } => {
                    println!(
                        "Downloaded Piece {}, Offset {}, Length {}",
                        index,
                        begin,
                        block.len()
                    );

                    return Ok(());
                }

                Message::Have(index) => {
                    println!("Peer have Piece {}", index);
                }

                _ => {
                    println!("Ignored Message: {:?}", msg);
                }
            }
        }
    }

    async fn request_loop(&mut self) -> anyhow::Result<()> {
        while !self.peer_choking {
            let piece_idx = match self.queue.pop() {
                Some(idx) => idx,
                None => {
                    println!("No more pieces to request");
                    return Ok(());
                }
            };

            if let Some(bf) = &self.bitfield {
                if !bf.has_piece(piece_idx) {
                    self.queue.push(piece_idx);
                    continue;
                }
            }

            println!("Downloading piece: {}", piece_idx);

            self.download_piece(piece_idx).await?;
        }

        Ok(())
    }

    async fn download_piece(&mut self, index: usize) -> anyhow::Result<()> {
        println!("Downloading Piece {}...", index);

        let begin_offset = index as u64 * self.piece_length as u64;
        let mut piece_len = self.piece_length;

        if begin_offset + piece_len as u64 > self.file_length {
            piece_len = (self.file_length - begin_offset) as u32;
        }

        let mut piece_buffer = vec![0u8; piece_len as usize];

        let mut downloaded = 0;
        let mut requested = 0;
        let mut backlog = 0;
        let max_backlog = 5;

        while downloaded < piece_len {
            while backlog < max_backlog && requested < piece_len {
                let remaining = piece_len - requested;
                let block_size = std::cmp::min(crate::constants::BLOCK_MAX, remaining);

                self.peer
                    .send_message(Message::Request {
                        index: index as u32,
                        begin: requested,
                        length: block_size,
                    })
                    .await?;

                backlog += 1;
                requested += block_size;
            }

            let msg = self.peer.next_message().await?;

            match msg {
                Message::Piece {
                    index: idx,
                    begin,
                    block,
                } => {
                    if idx as usize == index
                        && begin < piece_len
                        && (begin + block.len() as u32) <= piece_len
                    {
                        let begin_usize = begin as usize;
                        piece_buffer[begin_usize..begin_usize + block.len()]
                            .copy_from_slice(&block);

                        downloaded += block.len() as u32;
                        backlog -= 1;
                    } else {
                        // Ignore irrelevant blocks or log error
                    }
                }
                Message::Choke => {
                    anyhow::bail!("Peer choked during download");
                }
                _ => {}
            }
        }

        {
            let mut file = self.file.lock().await;
            file.seek(SeekFrom::Start(begin_offset)).await?;
            file.write_all(&piece_buffer).await?;
        }

        println!("Finished Piece {}", index);
        Ok(())
    }
    async fn send_interested(&mut self) -> anyhow::Result<()> {
        if !self.am_interested {
            self.am_interested = true;
            self.peer.send_message(Message::Interested).await?;
        }
        Ok(())
    }

    async fn request_piece(&mut self) -> anyhow::Result<()> {
        if self.peer_choking {
            return Ok(());
        }

        if let Some(bf) = &self.bitfield {
            for i in 0..bf.pieces() {
                if bf.has_piece(i) {
                    println!("Requesting Piece {}", i);

                    self.peer
                        .send_message(Message::Request {
                            index: i as u32,
                            begin: 0,
                            length: BLOCK_MAX,
                        })
                        .await?;

                    break;
                }
            }
        }

        Ok(())
    }
}
