//! Overlay UI state for the command browser modal.
//! Manages search query input and selected result index.

use crate::index::CommandEntry;

#[derive(Debug, Clone, Default)]
pub struct OverlayState {
    pub visible: bool,
    pub query: String,
    pub results: Vec<CommandEntry>,
    pub selected: usize,
}

impl OverlayState {
    /// Show the overlay and reset state.
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.results.clear();
        self.selected = 0;
    }

    /// Hide the overlay.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Append a character to the search query.
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    /// Remove last character from the search query.
    pub fn pop_char(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected = (self.selected + 1) % self.results.len();
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            self.selected = self.selected.wrapping_sub(1).min(self.results.len() - 1);
        }
    }

    /// Returns the currently selected command name for injection into PTY stdin.
    pub fn selected_command(&self) -> Option<&str> {
        self.results.get(self.selected).map(|e| e.name.as_str())
    }

    /// Update search results from a fresh query.
    pub fn update_results(&mut self, results: Vec<CommandEntry>) {
        self.results = results;
        self.selected = 0;
    }
}
