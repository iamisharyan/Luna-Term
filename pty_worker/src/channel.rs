// PTY channel utilities – bounded lock-free ring channels for PTY I/O

use crossbeam_channel::{bounded, Receiver, Sender};

/// Create a matched (writer, reader) pair for PTY byte chunks.
/// Buffer capacity is in number of chunks, not bytes.
pub fn pty_channel(capacity: usize) -> (Sender<Vec<u8>>, Receiver<Vec<u8>>) {
    bounded(capacity)
}
