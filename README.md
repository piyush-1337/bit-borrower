# BitBorrower 🦀

A minimalist BitTorrent client written in Rust

* **Protocol:** BEP 0003 (BitTorrent Specification)

---

## Implementation Checklist

- [ ] **Parse Bencode:** Implement logic to decode `.torrent` files and extract the info-hash and tracker URL.
- [ ] **Tracker Announce:** Send an HTTP/UDP request to the tracker to receive a list of active peers.
- [ ] **Peer Handshake:** Establish a TCP connection and perform the initial 68-byte BitTorrent handshake.
- [ ] **Message Framing:** Implement a length-prefixed message parser for the Peer Wire Protocol.
- [ ] **Bitfield Handling:** Track which pieces peers have and broadcast your own availability.
- [ ] **Choke/Unchoke Logic:** Manage the state machine for peer interest and choking status.
- [ ] **Request Blocks:** Implement the logic to request 16KB blocks from peers for a specific piece.
- [ ] **Piece Verification:** Use SHA-1 to verify the integrity of a downloaded piece against the torrent file's hash.
- [ ] **File I/O:** Write verified pieces to the disk and handle multi-file torrent layouts.
- [ ] **Concurrency:** Use `tokio` to manage multiple peer connections simultaneously.
- [ ] **CLI Interface:** Create a basic terminal dashboard to monitor download speed and progress.
- [ ] **Extension:** Add support for optional extensions.
- [ ] **μTP Protocol:** Implement the μTP protocol for efficient file sharing.
