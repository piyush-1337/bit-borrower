use std::sync::Mutex;

#[derive(Debug)]
pub struct WorkQueue {
    pieces: Mutex<Vec<usize>>,
}

impl WorkQueue {
    pub fn new(piece_count: usize) -> Self {
        let mut pieces: Vec<usize> = (0..piece_count).collect();
        pieces.reverse();

        Self {
            pieces: Mutex::new(pieces),
        }
    }

    pub fn pop(&self) -> Option<usize> {
        let mut pieces = self.pieces.lock().unwrap();
        pieces.pop()
    }

    pub fn push(&self, piece: usize) {
        let mut pieces = self.pieces.lock().unwrap();
        pieces.push(piece);
    }
}
