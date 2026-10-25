/// Theme — color palette for the terminal UI.
/// Designed to look beautiful and compete with Kitty/Warp.

#[derive(Debug, Clone)]
pub struct Rgba(pub f64, pub f64, pub f64, pub f64);

impl Rgba {
    pub fn to_wgpu(&self) -> wgpu::Color {
        wgpu::Color { r: self.0, g: self.1, b: self.2, a: self.3 }
    }
}

/// A complete terminal color theme.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub background: Rgba,
    pub foreground: Rgba,
    pub cursor: Rgba,
    pub selection_bg: Rgba,
    pub ansi: [Rgba; 16],       // Standard 16 ANSI colors
    pub ui_bar_bg: Rgba,        // Bottom status bar background
    pub ui_bar_fg: Rgba,        // Status bar text
    pub overlay_bg: Rgba,       // Command browser overlay background
    pub overlay_border: Rgba,   // Overlay border
    pub overlay_highlight: Rgba,// Selected item in overlay
}

impl Theme {
    /// "Luna Dark" — a beautiful dark theme designed to beat Kitty/Warp visually.
    pub fn luna_dark() -> Self {
        Theme {
            name: "Luna Dark",
            background: Rgba(0.047, 0.047, 0.063, 1.0),     // #0C0C10
            foreground: Rgba(0.867, 0.867, 0.918, 1.0),     // #DDDCEA
            cursor: Rgba(0.533, 0.494, 0.973, 1.0),         // #887DF8 — soft violet
            selection_bg: Rgba(0.267, 0.251, 0.482, 0.5),   // #44407B 50%
            ui_bar_bg: Rgba(0.067, 0.063, 0.094, 1.0),      // #111018
            ui_bar_fg: Rgba(0.533, 0.525, 0.620, 1.0),      // #88869E
            overlay_bg: Rgba(0.078, 0.075, 0.118, 0.96),    // #14131E ~96%
            overlay_border: Rgba(0.310, 0.290, 0.490, 1.0), // #4F4A7D
            overlay_highlight: Rgba(0.200, 0.188, 0.345, 1.0), // #333058
            ansi: [
                // Normal colors
                Rgba(0.071, 0.071, 0.078, 1.0),  // 0  black
                Rgba(0.937, 0.329, 0.384, 1.0),  // 1  red
                Rgba(0.337, 0.867, 0.502, 1.0),  // 2  green
                Rgba(0.988, 0.812, 0.369, 1.0),  // 3  yellow
                Rgba(0.357, 0.592, 0.976, 1.0),  // 4  blue
                Rgba(0.671, 0.502, 0.973, 1.0),  // 5  magenta
                Rgba(0.231, 0.839, 0.902, 1.0),  // 6  cyan
                Rgba(0.867, 0.867, 0.918, 1.0),  // 7  white
                // Bright colors
                Rgba(0.298, 0.290, 0.380, 1.0),  // 8  bright black
                Rgba(0.969, 0.494, 0.533, 1.0),  // 9  bright red
                Rgba(0.529, 0.953, 0.667, 1.0),  // 10 bright green
                Rgba(0.996, 0.882, 0.561, 1.0),  // 11 bright yellow
                Rgba(0.549, 0.714, 0.988, 1.0),  // 12 bright blue
                Rgba(0.796, 0.655, 0.988, 1.0),  // 13 bright magenta
                Rgba(0.404, 0.922, 0.957, 1.0),  // 14 bright cyan
                Rgba(0.961, 0.961, 0.988, 1.0),  // 15 bright white
            ],
        }
    }

    /// "Luna Light" — clean light theme.
    pub fn luna_light() -> Self {
        Theme {
            name: "Luna Light",
            background: Rgba(0.969, 0.969, 0.976, 1.0),
            foreground: Rgba(0.118, 0.118, 0.173, 1.0),
            cursor: Rgba(0.400, 0.353, 0.870, 1.0),
            selection_bg: Rgba(0.702, 0.682, 0.941, 0.4),
            ui_bar_bg: Rgba(0.918, 0.914, 0.957, 1.0),
            ui_bar_fg: Rgba(0.400, 0.388, 0.510, 1.0),
            overlay_bg: Rgba(0.969, 0.969, 0.992, 0.97),
            overlay_border: Rgba(0.600, 0.580, 0.780, 1.0),
            overlay_highlight: Rgba(0.851, 0.843, 0.961, 1.0),
            ansi: [
                Rgba(0.220, 0.220, 0.290, 1.0),
                Rgba(0.800, 0.160, 0.216, 1.0),
                Rgba(0.118, 0.600, 0.282, 1.0),
                Rgba(0.722, 0.557, 0.055, 1.0),
                Rgba(0.176, 0.404, 0.820, 1.0),
                Rgba(0.557, 0.278, 0.820, 1.0),
                Rgba(0.039, 0.620, 0.722, 1.0),
                Rgba(0.500, 0.500, 0.580, 1.0),
                Rgba(0.420, 0.420, 0.500, 1.0),
                Rgba(0.878, 0.259, 0.306, 1.0),
                Rgba(0.180, 0.718, 0.361, 1.0),
                Rgba(0.820, 0.655, 0.118, 1.0),
                Rgba(0.290, 0.518, 0.898, 1.0),
                Rgba(0.655, 0.380, 0.898, 1.0),
                Rgba(0.078, 0.718, 0.820, 1.0),
                Rgba(0.169, 0.169, 0.231, 1.0),
            ],
        }
    }
}
