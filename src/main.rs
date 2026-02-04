pub mod torrent;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::torrent::Torrent;

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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Info { file } => {
            println!("Parsing file: {}", file.display());

            let torrent = Torrent::read(&file)?;
            let info_hash = torrent.info.hash()?;

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
                println!("Error: Torrent info invalid (neither length nor files present)");
            }
        }
    }

    Ok(())
}
