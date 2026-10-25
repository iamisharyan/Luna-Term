//! ANSI / VT state machine (Paul Williams model).
//! Parses raw bytes into high-level `Action` events.

use crate::{action::Action, cell::{CellAttrs, Color}, grid::Grid};
use unicode_width::UnicodeWidthStr;

/// Internal parser state.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    OscString,
    DcsEntry,   // For APC/DCS (Kitty graphics, etc.)
    ApcString,  // Kitty graphics: ESC _ ... ESC \
}

pub struct Parser {
    pub grid: Grid,
    state: State,
    /// CSI parameter accumulator
    params: Vec<u16>,
    param_buf: String,
    /// Intermediate byte (e.g., '?' in CSI ? ...)
    intermediate: Option<u8>,
    /// OSC / APC string accumulator
    string_buf: Vec<u8>,
}

impl Parser {
    pub fn new(rows: usize, cols: usize) -> Self {
        Parser {
            grid: Grid::new(rows, cols),
            state: State::Ground,
            params: Vec::new(),
            param_buf: String::new(),
            intermediate: None,
            string_buf: Vec::new(),
        }
    }

    /// Feed raw bytes; returns a list of actions emitted.
    pub fn feed(&mut self, data: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();
        let text = String::from_utf8_lossy(data);
        let mut iter = text.chars().peekable();

        while let Some(ch) = iter.next() {
            let byte = ch as u8;
            match self.state {
                State::Ground => self.process_ground(ch, byte, &mut actions),
                State::Escape => self.process_escape(ch, byte, &mut iter, &mut actions),
                State::CsiEntry | State::CsiParam | State::CsiIntermediate => {
                    self.process_csi(ch, byte, &mut actions);
                }
                State::OscString => {
                    if byte == 0x07 || (byte == 0x1B && iter.peek() == Some(&'\\')) {
                        // OSC terminator
                        let _ = iter.next(); // consume '\\'
                        self.dispatch_osc(&mut actions);
                        self.state = State::Ground;
                    } else {
                        self.string_buf.push(byte);
                    }
                }
                State::ApcString => {
                    // Kitty graphics: accumulate until ESC \
                    if byte == 0x1B {
                        if iter.peek() == Some(&'\\') {
                            let _ = iter.next();
                            self.dispatch_apc(&mut actions);
                            self.state = State::Ground;
                        }
                    } else {
                        self.string_buf.push(byte);
                    }
                }
                _ => {
                    // Default: transition back to ground on unknown state
                    self.state = State::Ground;
                }
            }
        }

        // Apply all actions to the grid
        for action in &actions {
            self.apply(action.clone());
        }

        actions
    }

    fn process_ground(&mut self, ch: char, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x1B => {
                self.state = State::Escape;
            }
            0x0D => actions.push(Action::CarriageReturn),
            0x0A..=0x0C => actions.push(Action::LineFeed),
            0x08 => actions.push(Action::Backspace),
            0x07 => actions.push(Action::Bell),
            0x20..=0x7E | 0x80.. => {
                let g = ch.to_string();
                let _w = g.width().max(1);
                actions.push(Action::Print(g));
            }
            _ => {}
        }
    }

    fn process_escape(
        &mut self,
        _ch: char,
        byte: u8,
        _iter: &mut std::iter::Peekable<std::str::Chars>,
        actions: &mut Vec<Action>,
    ) {
        match byte {
            b'[' => {
                // CSI
                self.params.clear();
                self.param_buf.clear();
                self.intermediate = None;
                self.state = State::CsiEntry;
            }
            b']' => {
                // OSC
                self.string_buf.clear();
                self.state = State::OscString;
            }
            b'_' => {
                // APC — Kitty graphics
                self.string_buf.clear();
                self.state = State::ApcString;
            }
            b'7' => {
                actions.push(Action::SaveCursor);
                self.state = State::Ground;
            }
            b'8' => {
                actions.push(Action::RestoreCursor);
                self.state = State::Ground;
            }
            b'M' => {
                // Reverse index
                actions.push(Action::ScrollDown(1));
                self.state = State::Ground;
            }
            b'c' => {
                // RIS — Reset to initial state
                actions.push(Action::EraseInDisplay(2));
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn process_csi(&mut self, ch: char, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            b'0'..=b'9' => {
                self.param_buf.push(ch);
            }
            b';' => {
                self.flush_param();
            }
            b'?' | b'>' | b'!' => {
                self.intermediate = Some(byte);
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'@' | b'`' | b'~' => {
                self.flush_param();
                self.dispatch_csi(byte, actions);
                self.state = State::Ground;
            }
            _ => {
                self.state = State::CsiIgnore;
            }
        }
    }

    fn flush_param(&mut self) {
        let v: u16 = self.param_buf.parse().unwrap_or(0);
        self.params.push(v);
        self.param_buf.clear();
    }

    fn p(&self, idx: usize, default: u16) -> usize {
        let v = *self.params.get(idx).unwrap_or(&0);
        if v == 0 { default as usize } else { v as usize }
    }

    fn dispatch_csi(&mut self, final_byte: u8, actions: &mut Vec<Action>) {
        match final_byte {
            b'A' => actions.push(Action::CursorUp(self.p(0, 1))),
            b'B' => actions.push(Action::CursorDown(self.p(0, 1))),
            b'C' => actions.push(Action::CursorForward(self.p(0, 1))),
            b'D' => actions.push(Action::CursorBackward(self.p(0, 1))),
            b'H' | b'f' => {
                // CUP / HVP: row;col (1-indexed)
                let row = self.p(0, 1).saturating_sub(1);
                let col = self.p(1, 1).saturating_sub(1);
                actions.push(Action::CursorGoto { row, col });
            }
            b'J' => actions.push(Action::EraseInDisplay(self.params.first().copied().unwrap_or(0) as u8)),
            b'K' => actions.push(Action::EraseInLine(self.params.first().copied().unwrap_or(0) as u8)),
            b'L' => actions.push(Action::InsertLines(self.p(0, 1))),
            b'M' => actions.push(Action::DeleteLines(self.p(0, 1))),
            b'S' => actions.push(Action::ScrollUp(self.p(0, 1))),
            b'T' => actions.push(Action::ScrollDown(self.p(0, 1))),
            b'm' => actions.push(Action::SetGraphicsRendition(self.params.clone())),
            b'r' => {
                let top = self.p(0, 1).saturating_sub(1);
                let bottom = self.p(1, self.grid.rows as u16).saturating_sub(1);
                actions.push(Action::SetScrollRegion { top, bottom });
            }
            b'h' => {
                // DEC modes: e.g. ?25h = show cursor
                if self.intermediate == Some(b'?') {
                    if let Some(&25) = self.params.first() {
                        actions.push(Action::SetCursorVisible(true));
                    }
                }
            }
            b'l' => {
                if self.intermediate == Some(b'?') {
                    if let Some(&25) = self.params.first() {
                        actions.push(Action::SetCursorVisible(false));
                    }
                }
            }
            _ => {}
        }
        self.params.clear();
        self.param_buf.clear();
        self.intermediate = None;
    }

    fn dispatch_osc(&mut self, actions: &mut Vec<Action>) {
        if let Ok(s) = std::str::from_utf8(&self.string_buf) {
            if let Some(rest) = s.strip_prefix("0;").or_else(|| s.strip_prefix("2;")) {
                actions.push(Action::SetTitle(rest.to_string()));
            }
        }
        self.string_buf.clear();
    }

    fn dispatch_apc(&mut self, actions: &mut Vec<Action>) {
        // Kitty Graphics Protocol: ESC _ G <params> ; <data> ESC \
        if let Ok(s) = std::str::from_utf8(&self.string_buf) {
            if let Some(rest) = s.strip_prefix("G") {
                let parts: Vec<&str> = rest.splitn(2, ';').collect();
                let params = parts[0].to_string();
                let data = parts.get(1).map(|d| d.as_bytes().to_vec()).unwrap_or_default();
                actions.push(Action::KittyGraphics { params, data });
            }
        }
        self.string_buf.clear();
    }

    /// Apply a parsed action to the grid state.
    fn apply(&mut self, action: Action) {
        use Action::*;
        match action {
            Print(g) => {
                let w = g.width().max(1);
                self.grid.write_grapheme(&g, w);
            }
            CarriageReturn => self.grid.cursor.col = 0,
            LineFeed => self.grid.line_feed(),
            Backspace => {
                if self.grid.cursor.col > 0 {
                    self.grid.cursor.col -= 1;
                }
            }
            CursorGoto { row, col } => {
                self.grid.cursor.row = row.min(self.grid.rows.saturating_sub(1));
                self.grid.cursor.col = col.min(self.grid.cols.saturating_sub(1));
            }
            CursorUp(n) => self.grid.cursor.row = self.grid.cursor.row.saturating_sub(n),
            CursorDown(n) => {
                self.grid.cursor.row = (self.grid.cursor.row + n).min(self.grid.rows - 1);
            }
            CursorForward(n) => {
                self.grid.cursor.col = (self.grid.cursor.col + n).min(self.grid.cols - 1);
            }
            CursorBackward(n) => self.grid.cursor.col = self.grid.cursor.col.saturating_sub(n),
            EraseInDisplay(m) => self.grid.erase_in_display(m),
            EraseInLine(m) => self.grid.erase_in_line(m),
            InsertLines(n) => {
                let row = self.grid.cursor.row;
                let bot = self.grid.scroll_region.bottom;
                for _ in 0..n {
                    if bot < self.grid.rows - 1 {
                        self.grid.cells[row..=bot].rotate_right(1);
                        self.grid.cells[row] = vec![crate::cell::Cell::blank(); self.grid.cols];
                    }
                }
            }
            DeleteLines(n) => {
                let row = self.grid.cursor.row;
                let bot = self.grid.scroll_region.bottom;
                for _ in 0..n {
                    self.grid.cells[row..=bot].rotate_left(1);
                    self.grid.cells[bot] = vec![crate::cell::Cell::blank(); self.grid.cols];
                }
            }
            ScrollUp(n) => self.grid.scroll_up(n),
            ScrollDown(n) => self.grid.scroll_down(n),
            SetScrollRegion { top, bottom } => {
                self.grid.scroll_region.top = top.min(self.grid.rows - 1);
                self.grid.scroll_region.bottom = bottom.min(self.grid.rows - 1);
                self.grid.cursor.row = 0;
                self.grid.cursor.col = 0;
            }
            SaveCursor => self.grid.save_cursor(),
            RestoreCursor => self.grid.restore_cursor(),
            SetCursorVisible(v) => self.grid.cursor.visible = v,
            SetTitle(t) => self.grid.title = t,
            SetGraphicsRendition(params) => self.apply_sgr(&params),
            Bell | KittyGraphics { .. } => {} // handled by the app layer
        }
    }

    /// Apply SGR (Select Graphic Rendition) parameters.
    fn apply_sgr(&mut self, params: &[u16]) {
        let cursor = &mut self.grid.cursor;
        let mut i = 0;
        if params.is_empty() {
            // Reset all
            cursor.fg = Color::Default;
            cursor.bg = Color::Default;
            cursor.attrs = CellAttrs::empty();
            return;
        }
        while i < params.len() {
            match params[i] {
                0 => {
                    cursor.fg = Color::Default;
                    cursor.bg = Color::Default;
                    cursor.attrs = CellAttrs::empty();
                }
                1 => cursor.attrs |= CellAttrs::BOLD,
                2 => cursor.attrs |= CellAttrs::DIM,
                3 => cursor.attrs |= CellAttrs::ITALIC,
                4 => cursor.attrs |= CellAttrs::UNDERLINE,
                5 | 6 => cursor.attrs |= CellAttrs::BLINK,
                7 => cursor.attrs |= CellAttrs::REVERSE,
                9 => cursor.attrs |= CellAttrs::STRIKETHROUGH,
                22 => cursor.attrs &= !CellAttrs::BOLD,
                23 => cursor.attrs &= !CellAttrs::ITALIC,
                24 => cursor.attrs &= !CellAttrs::UNDERLINE,
                27 => cursor.attrs &= !CellAttrs::REVERSE,
                30..=37 => cursor.fg = Color::Indexed(params[i] as u8 - 30),
                38 => {
                    if params.get(i + 1) == Some(&5) && i + 2 < params.len() {
                        cursor.fg = Color::Indexed(params[i + 2] as u8);
                        i += 2;
                    } else if params.get(i + 1) == Some(&2) && i + 4 < params.len() {
                        cursor.fg = Color::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        );
                        i += 4;
                    }
                }
                39 => cursor.fg = Color::Default,
                40..=47 => cursor.bg = Color::Indexed(params[i] as u8 - 40),
                48 => {
                    if params.get(i + 1) == Some(&5) && i + 2 < params.len() {
                        cursor.bg = Color::Indexed(params[i + 2] as u8);
                        i += 2;
                    } else if params.get(i + 1) == Some(&2) && i + 4 < params.len() {
                        cursor.bg = Color::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        );
                        i += 4;
                    }
                }
                49 => cursor.bg = Color::Default,
                90..=97 => cursor.fg = Color::Indexed(params[i] as u8 - 90 + 8),
                100..=107 => cursor.bg = Color::Indexed(params[i] as u8 - 100 + 8),
                _ => {}
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_print() {
        let mut p = Parser::new(24, 80);
        let actions = p.feed(b"Hello");
        assert!(actions.iter().any(|a| matches!(a, Action::Print(s) if s == "H")));
    }

    #[test]
    fn test_cursor_movement() {
        let mut p = Parser::new(24, 80);
        p.feed(b"\x1b[5;10H"); // CUP row=5 col=10 (1-indexed)
        assert_eq!(p.grid.cursor.row, 4);
        assert_eq!(p.grid.cursor.col, 9);
    }

    #[test]
    fn test_sgr_bold_color() {
        let mut p = Parser::new(24, 80);
        p.feed(b"\x1b[1;31m"); // bold + red fg
        assert!(p.grid.cursor.attrs.contains(CellAttrs::BOLD));
        assert_eq!(p.grid.cursor.fg, Color::Indexed(1));
    }

    #[test]
    fn test_erase_in_line() {
        let mut p = Parser::new(24, 80);
        p.feed(b"Hello\x1b[1K"); // erase left of cursor
        // cursor should be at col 5, left side erased
        for c in 0..=5 {
            assert!(p.grid.cells[0][c].is_empty() || p.grid.cells[0][c].grapheme == " ");
        }
    }

    #[test]
    fn test_osc_title() {
        let mut p = Parser::new(24, 80);
        p.feed(b"\x1b]0;My Shell\x07");
        assert_eq!(p.grid.title, "My Shell");
    }
}
