use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    /// The filename. This is purely advisory.
    pub name: String,

    /// The number of bytes in each piece
    /// The last piece may have a different size
    #[serde(rename = "piece length")]
    pub piece_length: u64,

    /// String consisting of the concatenation of all 20-byte SHA1 hash values, one per piece (byte string, i.e. not urlencoded)
    /// Used to verify the integrity of the pieces
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,

    /// Length of the file in bytes
    /// Available only for single-file torrents
    pub length: Option<u64>,

    /// A list containing one or more `File` objects
    /// Available only for multi-file torrents
    pub files: Option<Vec<File>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    /// Length of the particular file in bytes
    pub length: u64,

    /// A list containing one or more string elements that together represent the path and filename.
    /// Each element in the list corresponds to either a directory name or (in the case of the final element) the filename
    pub path: Vec<String>,
}

impl Torrent {
    pub fn read(file_path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let file_content = std::fs::read(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.as_ref().display()))?;

        let torrent: Torrent =
            serde_bencode::from_bytes(&file_content).context("Failed to parse torrent file")?;

        Ok(torrent)
    }
}

impl Info {
    pub fn hash(&self) -> anyhow::Result<[u8; 20]> {
        let info_bytes =
            serde_bencode::to_bytes(self).context("Failed to serialize torrent info")?;

        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);

        Ok(hasher.finalize().into())
    }
}
