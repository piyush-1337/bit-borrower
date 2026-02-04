pub mod peer;
pub mod torrent;
pub mod tracker;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{
    torrent::Torrent,
    tracker::{TrackerRequest, TrackerResponse},
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
}

pub const PORT: u16 = 6881;
pub const PEER_ID: [u8; 20] = *b"-pT0001-123456789012";

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

        Commands::Handshake { file, peer } => {
            let torrent = Torrent::read(&file).await?;
            let info_hash = torrent.info.hash()?;

            let peer_addr: std::net::SocketAddrV4 =
                peer.parse().context("Failed to parse peer address")?;

            let mut peer = peer::Peer::new(peer_addr);

            match peer.handshake(info_hash, PEER_ID).await {
                Ok(peer_id) => println!("Remote Peer ID: {}", hex::encode(peer_id)),
                Err(e) => println!("Error: {}", e),
            }
        }
    }

    Ok(())
}
