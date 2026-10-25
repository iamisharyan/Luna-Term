// Unix PTY implementation using forkpty / termios
#![cfg(unix)]

use crate::{channel::pty_channel, ByteChunk, PlatformHandle, PseudoTerminal};
use std::io::{self, Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::thread;

pub struct UnixHandle {
    pub master_fd: RawFd,
    pub child_pid: libc::pid_t,
}

impl UnixHandle {
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), io::Error> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(self.master_fd, libc::TIOCSWINSZ, &ws) };
        if ret == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

pub fn spawn(shell: &str, rows: u16, cols: u16) -> Result<PseudoTerminal, io::Error> {
    use std::ffi::CString;

    let shell_c = CString::new(shell)?;
    let mut master_fd: RawFd = -1;

    let ws = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let pid = unsafe {
        libc::forkpty(
            &mut master_fd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &ws,
        )
    };

    match pid {
        -1 => Err(io::Error::last_os_error()),
        0 => {
            // Child process – exec the shell
            let argv: Vec<*const libc::c_char> = vec![shell_c.as_ptr(), std::ptr::null()];
            unsafe { libc::execvp(shell_c.as_ptr(), argv.as_ptr()) };
            std::process::exit(1);
        }
        child_pid => {
            // Parent process – set up channels
            let (stdin_tx, stdin_rx) = pty_channel(256);
            let (stdout_tx, stdout_rx) = pty_channel(256);

            // --- Reader thread: master_fd → stdout_tx ---
            let read_fd = master_fd;
            let tx = stdout_tx;
            thread::spawn(move || {
                let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
                let mut buf = [0u8; 4096];
                loop {
                    match file.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if tx.send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            });

            // --- Writer thread: stdin_rx → master_fd ---
            let write_fd = master_fd;
            thread::spawn(move || {
                let mut file = unsafe { std::fs::File::from_raw_fd(write_fd) };
                while let Ok(chunk) = stdin_rx.recv() {
                    if file.write_all(&chunk).is_err() {
                        break;
                    }
                }
            });

            Ok(PseudoTerminal {
                writer: stdin_tx,
                reader: stdout_rx,
                inner: PlatformHandle::Unix(UnixHandle {
                    master_fd,
                    child_pid,
                }),
            })
        }
    }
}
