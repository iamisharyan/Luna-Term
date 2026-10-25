//! PTY Worker — Platform-agnostic public API
//! Each platform has its own implementation module.

pub mod channel;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

use crossbeam_channel::{Receiver, Sender};
use std::io;

/// A chunk of raw bytes streamed from the child shell's stdout.
pub type ByteChunk = Vec<u8>;

/// A handle to the live PTY session.
pub struct PseudoTerminal {
    /// Send bytes to the child shell's stdin.
    pub writer: Sender<ByteChunk>,
    /// Receive raw byte chunks from the child shell's stdout.
    pub reader: Receiver<ByteChunk>,
    /// Internal platform handle (opaque).
    pub(crate) inner: PlatformHandle,
}

pub(crate) enum PlatformHandle {
    #[cfg(unix)]
    Unix(unix::UnixHandle),
    #[cfg(windows)]
    Windows(windows::WindowsHandle),
}

impl PseudoTerminal {
    /// Write bytes directly to the shell stdin.
    pub fn write(&self, data: &[u8]) -> Result<(), io::Error> {
        self.writer
            .send(data.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "PTY channel closed"))
    }

    /// Attempt to resize the terminal window.
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), io::Error> {
        match &mut self.inner {
            #[cfg(unix)]
            PlatformHandle::Unix(h) => h.resize(rows, cols),
            #[cfg(windows)]
            PlatformHandle::Windows(h) => h.resize(rows, cols),
        }
    }
}

/// Spawn a shell, returning a `PseudoTerminal` with async reader/writer channels.
/// `rows` and `cols` define the initial terminal size.
pub fn spawn_shell(shell: &str, rows: u16, cols: u16) -> Result<PseudoTerminal, io::Error> {
    #[cfg(unix)]
    {
        unix::spawn(shell, rows, cols)
    }
    #[cfg(windows)]
    {
        windows::spawn(shell, rows, cols)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unsupported platform",
        ))
    }
}
