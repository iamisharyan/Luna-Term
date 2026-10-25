//! Luna Terminal — main application entry point.
//! Wires together: PTY worker, VT parser, GPU renderer, and command browser.

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::WindowBuilder,
};

use command_browser::{Index, OverlayState};
use renderer::{theme::Theme, RendererSurface};
use vt_parser::{grid::GridSnapshot, Parser};

fn main() {
    env_logger::init();
    println!("=== Starting Luna Terminal ===");
    log::info!("Starting Luna Terminal");

    // -----------------------------------------------------------------------
    // 1. Spawn the shell PTY
    // -----------------------------------------------------------------------
    let shell = default_shell();
    println!("Spawning shell: {}", shell);
    log::info!("Spawning shell: {}", shell);
    let pty = pty_worker::spawn_shell(&shell, 24, 80)
        .expect("Failed to spawn shell PTY");
    println!("PTY spawned successfully");
    log::info!("PTY spawned successfully");

    let pty_writer = pty.writer.clone();
    let pty_reader = pty.reader.clone();

    // -----------------------------------------------------------------------
    // 2. VT Parser — runs on its own thread
    // -----------------------------------------------------------------------
    let grid_snapshot: Arc<Mutex<Option<GridSnapshot>>> = Arc::new(Mutex::new(None));
    let grid_snapshot_writer = Arc::clone(&grid_snapshot);

    thread::spawn(move || {
        let mut parser = Parser::new(24, 80);
        println!("PTY reader thread started");
        log::info!("PTY reader thread started");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            while let Ok(chunk) = pty_reader.recv() {
                println!("PTY received {} bytes", chunk.len());
                log::debug!("PTY received {} bytes: {:?}", chunk.len(), String::from_utf8_lossy(&chunk));
                parser.feed(&chunk);
                let snap = parser.grid.snapshot();
                if let Ok(mut lock) = grid_snapshot_writer.lock() {
                    *lock = Some(snap);
                }
            }
        }));
        if let Err(e) = result {
            println!("PTY reader thread panicked: {:?}", e);
            log::error!("PTY reader thread panicked: {:?}", e);
        }
        println!("PTY reader thread ended");
        log::warn!("PTY reader thread ended (channel closed)");
    });

    // -----------------------------------------------------------------------
    // 3. Command Browser Index
    // -----------------------------------------------------------------------
    let index = Rc::new(
        Index::in_memory().expect("Failed to create command index")
    );
    let overlay_state = Arc::new(Mutex::new(OverlayState::default()));

    // -----------------------------------------------------------------------
    // 4. winit 0.29 Event Loop + GPU Renderer
    // -----------------------------------------------------------------------
    let event_loop = EventLoop::new().expect("EventLoop creation failed");
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Luna Terminal")
            .with_inner_size(winit::dpi::LogicalSize::new(1200u32, 800u32))
            .with_decorations(true)
            .with_transparent(false)
            .with_visible(false)
            .build(&event_loop)
            .expect("Window creation failed"),
    );

    let theme = Theme::luna_dark();
    let cell_w = 8.0f32;
    let cell_h = 16.0f32;

    let mut surface = pollster::block_on(
        RendererSurface::new(Arc::clone(&window), theme, cell_w, cell_h)
    );

    let initial_snapshot = grid_snapshot.lock().unwrap().clone().unwrap_or_else(|| {
        Parser::new(24, 80).grid.snapshot()
    });
    surface.draw(&initial_snapshot);
    window.set_visible(true);

    let mut modifiers = ModifiersState::empty();
    let _ = event_loop.run(move |event, elwt| {
        match event {
            // ------------------------------------------------------------------
            // Window close
            // ------------------------------------------------------------------
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }

            // ------------------------------------------------------------------
            // Resize
            // ------------------------------------------------------------------
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                surface.resize(size.width, size.height);
            }

            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(state),
                ..
            } => {
                modifiers = state.state();
            }

            // ------------------------------------------------------------------
            // Keyboard input (winit 0.29 unified KeyEvent)
            // ------------------------------------------------------------------
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event: ref key_event, .. },
                ..
            } if key_event.state == ElementState::Pressed => {
                let mut overlay = overlay_state.lock().unwrap();

                // Ctrl+Space → toggle command browser
                if is_browser_toggle(&key_event.logical_key, modifiers) {
                    if overlay.visible { overlay.hide(); } else { overlay.show(); }
                    return;
                }

                if overlay.visible {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::Escape) => overlay.hide(),
                        Key::Named(NamedKey::Enter) => {
                            if let Some(cmd) = overlay.selected_command() {
                                let payload = format!("{}\n", cmd);
                                let _ = pty_writer.send(payload.into_bytes());
                            }
                            overlay.hide();
                        }
                        Key::Named(NamedKey::ArrowUp) => overlay.select_prev(),
                        Key::Named(NamedKey::ArrowDown) => overlay.select_next(),
                        Key::Named(NamedKey::Backspace) => {
                            overlay.pop_char();
                            let results = index.search(&overlay.query, 20).unwrap_or_default();
                            overlay.update_results(results);
                        }
                        Key::Character(s) => {
                            for ch in s.chars() {
                                if !ch.is_control() {
                                    overlay.push_char(ch);
                                }
                            }
                            let results = index.search(&overlay.query, 20).unwrap_or_default();
                            overlay.update_results(results);
                        }
                        _ => {}
                    }
                } else {
                    // Pass through to PTY
                    if let Some(bytes) = key_to_ansi(&key_event.logical_key) {
                        let _ = pty_writer.send(bytes);
                    }
                }
            }

            // ------------------------------------------------------------------
            // Main draw
            // ------------------------------------------------------------------
            Event::AboutToWait => {
                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let snap = grid_snapshot.lock().unwrap().clone();
                let snapshot = snap.unwrap_or_else(|| {
                    // Initial blank grid snapshot before PTY produces output
                    let parser = Parser::new(24, 80);
                    parser.grid.snapshot()
                });
                surface.draw(&snapshot);
            }

            _ => {}
        }
    });
}

/// Map a winit logical key to an ANSI escape byte sequence.
fn key_to_ansi(key: &winit::keyboard::Key) -> Option<Vec<u8>> {
    use winit::keyboard::{Key, NamedKey};
    match key {
        Key::Named(NamedKey::Space)     => Some(b" ".to_vec()),
        Key::Named(NamedKey::Enter)     => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Backspace) => Some(b"\x7f".to_vec()),
        Key::Named(NamedKey::Tab)       => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Escape)    => Some(b"\x1b".to_vec()),
        Key::Named(NamedKey::ArrowUp)   => Some(b"\x1b[A".to_vec()),
        Key::Named(NamedKey::ArrowDown) => Some(b"\x1b[B".to_vec()),
        Key::Named(NamedKey::ArrowRight)=> Some(b"\x1b[C".to_vec()),
        Key::Named(NamedKey::ArrowLeft) => Some(b"\x1b[D".to_vec()),
        Key::Named(NamedKey::Home)      => Some(b"\x1b[H".to_vec()),
        Key::Named(NamedKey::End)       => Some(b"\x1b[F".to_vec()),
        Key::Named(NamedKey::PageUp)    => Some(b"\x1b[5~".to_vec()),
        Key::Named(NamedKey::PageDown)  => Some(b"\x1b[6~".to_vec()),
        Key::Named(NamedKey::Delete)    => Some(b"\x1b[3~".to_vec()),
        Key::Named(NamedKey::F1)        => Some(b"\x1bOP".to_vec()),
        Key::Named(NamedKey::F2)        => Some(b"\x1bOQ".to_vec()),
        Key::Named(NamedKey::F3)        => Some(b"\x1bOR".to_vec()),
        Key::Named(NamedKey::F4)        => Some(b"\x1bOS".to_vec()),
        Key::Named(NamedKey::F5)        => Some(b"\x1b[15~".to_vec()),
        Key::Named(NamedKey::F6)        => Some(b"\x1b[17~".to_vec()),
        Key::Named(NamedKey::F7)        => Some(b"\x1b[18~".to_vec()),
        Key::Named(NamedKey::F8)        => Some(b"\x1b[19~".to_vec()),
        Key::Named(NamedKey::F9)        => Some(b"\x1b[20~".to_vec()),
        Key::Named(NamedKey::F10)       => Some(b"\x1b[21~".to_vec()),
        Key::Named(NamedKey::F11)       => Some(b"\x1b[23~".to_vec()),
        Key::Named(NamedKey::F12)       => Some(b"\x1b[24~".to_vec()),
        Key::Character(s)               => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

fn is_browser_toggle(key: &Key, modifiers: ModifiersState) -> bool {
    modifiers.control_key() && key == &Key::Named(NamedKey::Space)
}

#[cfg(test)]
mod tests {
    use super::{is_browser_toggle, key_to_ansi};
    use winit::keyboard::{Key, ModifiersState, NamedKey};

    #[test]
    fn space_is_sent_to_the_shell_unless_control_is_held() {
        let space = Key::Named(NamedKey::Space);
        assert_eq!(key_to_ansi(&space), Some(b" ".to_vec()));
        assert!(!is_browser_toggle(&space, ModifiersState::empty()));
        assert!(is_browser_toggle(&space, ModifiersState::CONTROL));
    }
}

fn default_shell() -> String {
    #[cfg(windows)]
    {
        // Just cmd.exe, portable-pty will run it interactively
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
    #[cfg(not(windows))]
    { std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()) }
}
