// Windows ConPTY using portable-pty with concrete Read/Write from MasterPty
#![cfg(windows)]

use crate::{channel::pty_channel, PlatformHandle, PseudoTerminal};
use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use crossbeam_channel::{bounded, select};
use log::debug;

pub struct WindowsHandle {
    pub resize_tx: crossbeam_channel::Sender<(u16, u16)>,
    pub process: Box<dyn portable_pty::Child + Send + Sync>,
}

impl WindowsHandle {
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), io::Error> {
        self.resize_tx.send((rows, cols))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Resize channel closed"))
    }
}

pub fn spawn(shell: &str, rows: u16, cols: u16) -> Result<PseudoTerminal, io::Error> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let cmd = CommandBuilder::new(shell);
    let child = pty_pair.slave.spawn_command(cmd)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    debug!("Process spawned with PID: {:?}", child.process_id());

    // Get reader and writer from MasterPty trait methods
    let reader = pty_pair.master.try_clone_reader()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let writer = pty_pair.master.take_writer()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Channels for communication with main app
    let (stdout_tx, stdout_rx) = pty_channel(256);
    let (stdin_tx, stdin_rx) = pty_channel(256);
    let (resize_tx, resize_rx) = bounded(1);

    // Keep the master for resize operations
    let master = pty_pair.master;

    // Single I/O thread handling read, write, and resize
    thread::spawn(move || {
        debug!("PTY I/O thread started");
        let mut reader = reader;
        let mut writer = writer;
        let mut buf = [0u8; 4096];

        loop {
            select! {
                recv(stdin_rx) -> msg => {
                    match msg {
                        Ok(chunk) => {
                            if let Err(e) = writer.write_all(&chunk) {
                                debug!("PTY write error: {}", e);
                                break;
                            }
                            let _ = writer.flush();
                        }
                        Err(_) => {
                            debug!("PTY stdin channel closed");
                            break;
                        }
                    }
                }
                recv(resize_rx) -> msg => {
                    match msg {
                        Ok((rows, cols)) => {
                            let _ = master.resize(PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                        Err(_) => {
                            debug!("Resize channel closed");
                        }
                    }
                }
                default(Duration::from_millis(10)) => {
                    // Try reading from PTY
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            debug!("PTY read EOF");
                            break;
                        }
                        Ok(n) => {
                            debug!("PTY read {} bytes", n);
                            if stdout_tx.send(buf[..n].to_vec()).is_err() {
                                debug!("PTY stdout channel closed");
                                break;
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // No data available
                        }
                        Err(e) => {
                            debug!("PTY read error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
        debug!("PTY I/O thread ended");
    });

    Ok(PseudoTerminal {
        writer: stdin_tx,
        reader: stdout_rx,
        inner: PlatformHandle::Windows(WindowsHandle {
            resize_tx,
            process: child,
        }),
    })
}