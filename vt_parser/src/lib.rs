//! vt_parser — ANSI/VT state machine, grid buffer, and action types.

pub mod action;
pub mod cell;
pub mod grid;
pub mod parser;

pub use action::Action;
pub use cell::{Cell, CellAttrs, Color};
pub use grid::Grid;
pub use parser::Parser;
