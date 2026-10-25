//! renderer — GPU-accelerated terminal renderer using wgpu.
//! Pipeline: Grid snapshot → cell vertices → GPU texture atlas draw.

pub mod atlas;
pub mod pipeline;
pub mod surface;
pub mod theme;

pub use surface::RendererSurface;
pub use theme::Theme;
