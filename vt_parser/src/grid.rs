//! Grid — 2D viewport + ring-buffer scrollback.

use crate::cell::{Cell, CellAttrs, Color};

/// Cursor state
#[derive(Debug, Clone)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub fg: Color,
    pub bg: Color,
    pub attrs: CellAttrs,
    pub visible: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            row: 0,
            col: 0,
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttrs::empty(),
            visible: true,
        }
    }
}

/// Scroll region (inclusive row indices)
#[derive(Debug, Clone, Copy)]
pub struct ScrollRegion {
    pub top: usize,
    pub bottom: usize,
}

/// The full terminal grid state.
pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    /// Active viewport — row-major. Access as `cells[row][col]`.
    pub cells: Vec<Vec<Cell>>,
    /// Scrollback ring buffer.
    pub scrollback: std::collections::VecDeque<Vec<Cell>>,
    pub scrollback_limit: usize,
    pub cursor: Cursor,
    pub saved_cursor: Option<Cursor>,
    pub scroll_region: ScrollRegion,
    pub title: String,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        let cells = vec![vec![Cell::blank(); cols]; rows];
        Grid {
            rows,
            cols,
            cells,
            scrollback: std::collections::VecDeque::with_capacity(10_000),
            scrollback_limit: 10_000,
            cursor: Cursor::default(),
            saved_cursor: None,
            scroll_region: ScrollRegion { top: 0, bottom: rows.saturating_sub(1) },
            title: String::from("Luna Terminal"),
        }
    }

    /// Resize the grid, preserving content where possible.
    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.cells.resize_with(rows, || vec![Cell::blank(); cols]);
        for row in &mut self.cells {
            row.resize_with(cols, Cell::blank);
        }
        self.rows = rows;
        self.cols = cols;
        self.scroll_region = ScrollRegion { top: 0, bottom: rows.saturating_sub(1) };
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
    }

    /// Write a grapheme at the current cursor position and advance.
    pub fn write_grapheme(&mut self, grapheme: &str, width: usize) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        if col < self.cols {
            self.cells[row][col] = Cell {
                grapheme: grapheme.to_string(),
                fg: self.cursor.fg,
                bg: self.cursor.bg,
                attrs: self.cursor.attrs,
                wide_placeholder: false,
            };
            // Mark second cell for wide chars
            if width == 2 && col + 1 < self.cols {
                self.cells[row][col + 1] = Cell {
                    grapheme: String::new(),
                    fg: self.cursor.fg,
                    bg: self.cursor.bg,
                    attrs: self.cursor.attrs,
                    wide_placeholder: true,
                };
            }
        }
        self.cursor.col = (col + width).min(self.cols);
    }

    /// Line feed — scroll if at bottom of scroll region.
    pub fn line_feed(&mut self) {
        if self.cursor.row == self.scroll_region.bottom {
            self.scroll_up(1);
        } else {
            self.cursor.row = (self.cursor.row + 1).min(self.rows - 1);
        }
    }

    /// Scroll up N rows within the scroll region; push evicted rows to scrollback.
    pub fn scroll_up(&mut self, n: usize) {
        let top = self.scroll_region.top;
        let bot = self.scroll_region.bottom;
        for _ in 0..n {
            let evicted = self.cells[top].clone();
            if self.scrollback.len() >= self.scrollback_limit {
                self.scrollback.pop_front();
            }
            self.scrollback.push_back(evicted);
            self.cells[top..=bot].rotate_left(1);
            self.cells[bot] = vec![Cell::blank(); self.cols];
        }
    }

    /// Scroll down N rows within the scroll region.
    pub fn scroll_down(&mut self, n: usize) {
        let top = self.scroll_region.top;
        let bot = self.scroll_region.bottom;
        for _ in 0..n {
            self.cells[top..=bot].rotate_right(1);
            self.cells[top] = vec![Cell::blank(); self.cols];
        }
    }

    /// Erase in display: 0=below, 1=above, 2=all, 3=all+scrollback.
    pub fn erase_in_display(&mut self, mode: u8) {
        let blank = Cell::blank();
        match mode {
            0 => {
                let row = self.cursor.row;
                let col = self.cursor.col;
                for c in col..self.cols { self.cells[row][c] = blank.clone(); }
                for r in (row + 1)..self.rows {
                    self.cells[r] = vec![blank.clone(); self.cols];
                }
            }
            1 => {
                let row = self.cursor.row;
                let col = self.cursor.col;
                for r in 0..row { self.cells[r] = vec![blank.clone(); self.cols]; }
                for c in 0..=col { self.cells[row][c] = blank.clone(); }
            }
            2 | 3 => {
                for r in 0..self.rows { self.cells[r] = vec![blank.clone(); self.cols]; }
                if mode == 3 { self.scrollback.clear(); }
                self.cursor.row = 0;
                self.cursor.col = 0;
            }
            _ => {}
        }
    }

    /// Erase in line: 0=right, 1=left, 2=all.
    pub fn erase_in_line(&mut self, mode: u8) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let blank = Cell::blank();
        match mode {
            0 => for c in col..self.cols { self.cells[row][c] = blank.clone(); },
            1 => for c in 0..=col { self.cells[row][c] = blank.clone(); },
            2 => self.cells[row] = vec![blank.clone(); self.cols],
            _ => {}
        }
    }

    /// Save current cursor state.
    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor.clone());
    }

    /// Restore saved cursor state.
    pub fn restore_cursor(&mut self) {
        if let Some(saved) = self.saved_cursor.take() {
            self.cursor = saved;
        }
    }

    /// Take a snapshot of the grid for the renderer (cheap clone).
    pub fn snapshot(&self) -> GridSnapshot {
        GridSnapshot {
            rows: self.rows,
            cols: self.cols,
            cells: self.cells.clone(),
            cursor: self.cursor.clone(),
            title: self.title.clone(),
        }
    }
}

/// Immutable snapshot of the grid, safe to send across threads.
#[derive(Debug, Clone)]
pub struct GridSnapshot {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<Cell>>,
    pub cursor: Cursor,
    pub title: String,
}
