//! Glyph rasterization and glyph bitmap extraction.

use crate::font::Font;

/// Rasterized 8-bit alpha coverage mask and placement metrics for a glyph.
#[derive(Debug, Clone)]
pub struct GlyphBitmap {
    pub width: usize,
    pub height: usize,
    pub xmin: i32,
    pub ymin: i32,
    pub advance_width: f32,
    pub alpha_mask: Vec<u8>,
}

impl Font {
    /// Rasterizes a single character glyph into an 8-bit alpha mask at the given font size.
    pub fn rasterize_glyph(&self, c: char, font_size_px: f32) -> GlyphBitmap {
        let (metrics, alpha_mask) = self.inner().rasterize(c, font_size_px);
        GlyphBitmap {
            width: metrics.width,
            height: metrics.height,
            xmin: metrics.xmin,
            ymin: metrics.ymin,
            advance_width: metrics.advance_width,
            alpha_mask,
        }
    }
}
