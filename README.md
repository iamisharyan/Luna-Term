# Luna Terminal

Luna is a Rust‑based terminal emulator that uses a **ConPTY** backend on Windows, a pseudoterminal on Unix, and the wgpu rendering engine for high‑performance GPU drawing. 

The repository contains several crates:  `terminal_emu`, `pty_worker`, `vt_parser`, `renderer`, and `command_browser`.  See the [README](#luna-terminal) for details and installation instructions.

## Prerequisites
- Rust 1.74+ (install via rustup)
- C++ build tools (MSVC on Windows, make/gcc on Linux/macOS)

## Building & Running
```bash
# Clone the repo
git clone https://github.com/iamisharyan/Luna-Term.git
cd Luna-Term

# Build
cargo build --release -p terminal_emu

# Run
./target/release/luna
```

## Quick‑start scripts
- Windows: `install_windows.bat`
- Linux: `install_linux.sh`
- macOS: `install_macos.sh`

## Contributing
Feel free to open issues or PRs.
