//! GPU texture atlas for glyph caching.
//! Each rendered glyph is rasterized once and stored as a sub-region of a large texture.

/// Axis-aligned rectangle within the atlas texture (in pixels).
#[derive(Debug, Clone, Copy)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A single glyph entry in the atlas.
#[derive(Debug, Clone)]
pub struct GlyphEntry {
    pub rect: AtlasRect,
    /// Horizontal advance in pixels (for layout).
    pub advance: f32,
    /// Bearing (offset from cursor origin to top-left of glyph box).
    pub bearing_x: f32,
    pub bearing_y: f32,
}

/// The atlas manages a 2D bin-packing of glyphs on the GPU texture.
/// Currently uses a simple shelf-packing algorithm.
pub struct Atlas {
    pub width: u32,
    pub height: u32,
    /// Current shelf cursor
    shelf_x: u32,
    shelf_y: u32,
    shelf_h: u32,
    pub glyphs: std::collections::HashMap<String, GlyphEntry>,
    /// Raw RGBA pixel buffer for upload to GPU.
    pub pixels: Vec<u8>,
}

impl Atlas {
    pub fn new(width: u32, height: u32) -> Self {
        Atlas {
            width,
            height,
            shelf_x: 0,
            shelf_y: 0,
            shelf_h: 0,
            glyphs: std::collections::HashMap::new(),
            pixels: vec![0u8; (width * height * 4) as usize],
        }
    }

    /// Insert a rasterized glyph bitmap (RGBA) into the atlas.
    /// Returns the atlas rect or None if the atlas is full.
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &mut self,
        key: &str,
        bitmap: &[u8],
        w: u32,
        h: u32,
        advance: f32,
        bx: f32,
        by: f32,
    ) -> Option<AtlasRect> {
        if self.glyphs.contains_key(key) {
            return Some(self.glyphs[key].rect);
        }

        // Advance to next shelf if needed
        if self.shelf_x + w > self.width {
            self.shelf_y += self.shelf_h + 1;
            self.shelf_x = 0;
            self.shelf_h = 0;
        }
        if self.shelf_y + h > self.height {
            return None; // Atlas full — would need atlas growth (TODO)
        }

        // Copy bitmap into pixel buffer
        for row in 0..h {
            for col in 0..w {
                let src_idx = ((row * w + col) * 4) as usize;
                let dst_x = self.shelf_x + col;
                let dst_y = self.shelf_y + row;
                let dst_idx = ((dst_y * self.width + dst_x) * 4) as usize;
                if src_idx + 3 < bitmap.len() && dst_idx + 3 < self.pixels.len() {
                    self.pixels[dst_idx..dst_idx + 4]
                        .copy_from_slice(&bitmap[src_idx..src_idx + 4]);
                }
            }
        }

        let rect = AtlasRect { x: self.shelf_x, y: self.shelf_y, w, h };
        self.glyphs.insert(key.to_string(), GlyphEntry { rect, advance, bearing_x: bx, bearing_y: by });
        self.shelf_x += w + 1;
        self.shelf_h = self.shelf_h.max(h);

        Some(rect)
    }

    /// Lookup a cached glyph.
    pub fn get(&self, key: &str) -> Option<&GlyphEntry> {
        self.glyphs.get(key)
    }
}
