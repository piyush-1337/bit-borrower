pub mod constants;
pub mod message;
pub mod peer;
pub mod torrent;
pub mod tracker;
pub mod download;
pub mod queue;

use clap::{Parser, Subcommand};
use std::{path::PathBuf};

use crate::{
    constants::{PEER_ID, PORT}, download::DownloadWorker, peer::Peer, queue::WorkQueue, torrent::Torrent, tracker::{TrackerRequest, TrackerResponse}
};

#[derive(Parser)]
#[command(name = "BitBorrower")]
#[command(about = "A BitTorrent client")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Info {
        #[arg(short, long)]
        file: PathBuf,
    },

    SendRequest {
        #[arg(short, long)]
        file: PathBuf,
    },

    GetPeers {
        #[arg(short, long)]
        file: PathBuf,
    },

    Handshake {
        #[arg(short, long)]
        file: PathBuf,

        #[arg(short, long)]
        peer: String,
    },

    Download {
        #[arg(short, long)]
        file: PathBuf,

        #[arg(short, long)]
        output: Option<PathBuf>,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Info { file } => {
            let torrent = Torrent::read(&file).await?;
            let info_hash = torrent.info.hash()?;

            println!("Parsing file: {}", file.display());

            println!("--- Torrent Metadata ---");
            println!("Tracker URL: {}", torrent.announce);
            println!("File Name:   {}", torrent.info.name);
            println!("Piece Len:   {} bytes", torrent.info.piece_length);

            println!("Info Hash:   {}", hex::encode(info_hash));

            if !torrent.info.pieces.len().is_multiple_of(20) {
                anyhow::bail!("Invalid piece length: must be divisible by 20");
            }

            println!("Piece count: {}", torrent.info.pieces.len() / 20);

            if let Some(files) = &torrent.info.files {
                println!("--- Mode: Multi-File ---");
                println!("Directory: {}", torrent.info.name);

                for (i, file) in files.iter().enumerate() {
                    let path = file.path.join("/");
                    println!("  File #{}: {} ({} bytes)", i + 1, path, file.length);
                }
            } else if let Some(length) = torrent.info.length {
                println!("--- Mode: Single-File ---");
                println!("  File:   {} ({} bytes)", torrent.info.name, length);
            } else {
                anyhow::bail!("Error: Torrent info invalid (neither length nor files present)");
            }
        }

        Commands::SendRequest { file } => {
            let torrent = Torrent::read(&file).await?;
            let info_hash = torrent.info.hash()?;

            let request = TrackerRequest::new(&torrent, &info_hash, &PEER_ID, PORT);

            let url_param = request.as_query_string();
            let tracker_url = format!("{}?{}", torrent.announce, url_param);

            println!("Tracker URL: {}", tracker_url);

            let response = reqwest::get(&tracker_url).await?;
            let body = &response.bytes().await?;

            println!("Response Status: Success");
            println!("Response Body: {:?}", body);
        }

        Commands::GetPeers { file } => {
            let torrent = Torrent::read(&file).await?;
            let info_hash = torrent.info.hash()?;

            let request = TrackerRequest::new(&torrent, &info_hash, &PEER_ID, PORT);

            let url_param = request.as_query_string();
            let tracker_url = format!("{}?{}", torrent.announce, url_param);

            println!("Tracker URL: {}", tracker_url);

            let response = reqwest::get(&tracker_url).await?;
            let body_bytes = &response.bytes().await?;

            let tracker_response = TrackerResponse::from_bytes(body_bytes)?;

            if let Some(fail_reason) = tracker_response.failure_reason {
                anyhow::bail!("Failure reason: {}", fail_reason);
            }

            let peers = tracker_response.get_peers()?;

            println!("Found {} peers", peers.len());

            for (i, peer) in peers.iter().enumerate() {
                println!("Peer #{}: {:?}", i + 1, peer);
            }
        }

        // Commands::Handshake { file, peer } => {
        //     let torrent = Torrent::read(&file).await?;
        //     let info_hash = torrent.info.hash()?;
        //     let peer_addr: std::net::SocketAddrV4 = peer.parse()?;
        //
        //     println!("Connecting to peer: {}", peer);
        //     let mut peer = Peer::new(peer_addr);
        //     let remote_pid = peer.handshake(info_hash, PEER_ID).await?;
        //     println!("Connected to Peer ID: {}", hex::encode(remote_pid));
        //
        //     let worker = DownloadWorker::new(peer);
        //     worker.start().await?;
        // }

        Commands::Download { file, output } => {
            let torrent = Torrent::read(&file).await?;
            let info_hash = torrent.info.hash()?;
            let piece_count = torrent.info.pieces.len() / 20;

            println!("Downloading file: {}", torrent.info.name);
            println!("Starting download of {} pieces", piece_count);

            let queue = std::sync::Arc::new(WorkQueue::new(piece_count));

            let output_path = output.unwrap_or_else(|| PathBuf::from(&torrent.info.name));
            let file = tokio::fs::File::create(&output_path).await?;
            let shared_file = std::sync::Arc::new(tokio::sync::Mutex::new(file));

            let request = TrackerRequest::new(&torrent, &info_hash, &constants::PEER_ID, constants::PORT);
            let tracker_url = format!("{}?{}", torrent.announce, request.as_query_string());
            let res = reqwest::get(&tracker_url).await?.bytes().await?;
            let tracker_response: TrackerResponse = serde_bencode::from_bytes(&res)?;
            
            let peers = tracker_response.get_peers()?;
            let peers_len = peers.len();

            println!("Found {} peers", peers.len());

            let mut handles = Vec::new();

            let piece_length = torrent.info.piece_length as u32;
            let file_length = torrent.info.length.unwrap();
    
            for peer_info in peers.into_iter().take(std::cmp::min(20, peers_len)) {
                let queue = queue.clone();
                let file = shared_file.clone();
                let info_hash = info_hash.clone();

                let handle = tokio::spawn(async move {
                    println!("Connecting to {}", peer_info.addr);

                    let mut peer = Peer::new(peer_info.addr);

                    if let Err(e) = peer.handshake(info_hash, constants::PEER_ID).await {
                        println!("Failed to handshake with peer: {}", e);
                        return;
                    }

                    let worker = DownloadWorker::new(peer, queue, file, piece_length, file_length);
                    if let Err(e) = worker.start().await {
                        println!("Worker {} died: {}", peer_info.addr, e);
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.await?;
            }

            println!("Download Complete")
        }

        _ => anyhow::bail!("Unknown command"),
    }

    Ok(())
}
