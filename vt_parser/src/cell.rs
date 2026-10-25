//! Cell — a single terminal cell holding a grapheme, colors and attributes.
//! 24-bit RGB color or one of the standard 256/16 palette indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

bitflags::bitflags! {
    /// Text attribute flags (bold, italic, underline, etc.)
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CellAttrs: u16 {
        const BOLD          = 0b0000_0001;
        const ITALIC        = 0b0000_0010;
        const UNDERLINE     = 0b0000_0100;
        const BLINK         = 0b0000_1000;
        const REVERSE       = 0b0001_0000;
        const STRIKETHROUGH = 0b0010_0000;
        const DIM           = 0b0100_0000;
    }
}

/// A single terminal cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The grapheme cluster displayed in this cell.
    /// For double-width chars the second cell stores an empty string.
    pub grapheme: String,
    pub fg: Color,
    pub bg: Color,
    pub attrs: CellAttrs,
    /// True if this is the right half of a double-width character.
    pub wide_placeholder: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            grapheme: " ".to_string(),
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttrs::empty(),
            wide_placeholder: false,
        }
    }
}

impl Cell {
    pub fn blank() -> Self { Self::default() }

    pub fn is_empty(&self) -> bool {
        self.grapheme == " " || self.grapheme.is_empty()
    }
}
