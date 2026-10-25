/// Terminal actions emitted by the ANSI state machine.
/// The renderer and grid manager consume these.

#[derive(Debug, Clone)]
pub enum Action {
    /// Print a grapheme cluster at the current cursor position.
    Print(String),
    /// Move cursor to absolute position (row, col), 0-indexed.
    CursorGoto { row: usize, col: usize },
    /// Move cursor up by N rows.
    CursorUp(usize),
    /// Move cursor down by N rows.
    CursorDown(usize),
    /// Move cursor forward (right) by N cols.
    CursorForward(usize),
    /// Move cursor backward (left) by N cols.
    CursorBackward(usize),
    /// Carriage return — move cursor to column 0.
    CarriageReturn,
    /// Line feed — move cursor down one row, scroll if needed.
    LineFeed,
    /// Backspace — move cursor one step left (no erase).
    Backspace,
    /// Erase in display.  0=below, 1=above, 2=all, 3=all+scrollback.
    EraseInDisplay(u8),
    /// Erase in line.  0=right, 1=left, 2=all.
    EraseInLine(u8),
    /// Set Graphic Rendition (SGR): update fg, bg, bold, italic, etc.
    SetGraphicsRendition(Vec<u16>),
    /// Set terminal title via OSC 2.
    SetTitle(String),
    /// Bell (BEL).
    Bell,
    /// Insert N blank lines at cursor.
    InsertLines(usize),
    /// Delete N lines at cursor.
    DeleteLines(usize),
    /// Scroll up N rows.
    ScrollUp(usize),
    /// Scroll down N rows.
    ScrollDown(usize),
    /// Set scroll region (top, bottom), 0-indexed.
    SetScrollRegion { top: usize, bottom: usize },
    /// Save cursor position.
    SaveCursor,
    /// Restore cursor position.
    RestoreCursor,
    /// Show/hide the cursor.
    SetCursorVisible(bool),
    /// Kitty Graphics Protocol payload (raw parameter + data bytes).
    KittyGraphics { params: String, data: Vec<u8> },
}
