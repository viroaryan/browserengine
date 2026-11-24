//! Glyph rasterization and anti-aliased font blitting onto Canvas.

use crate::font::Font;
use nirvana_core::color::Rgba;
use nirvana_gfx::canvas::Canvas;

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

/// Renders a text string with anti-aliasing onto a `Canvas`.
///
/// `(origin_x, origin_y)` specifies the top-left coordinate of the text layout box.
pub fn draw_text(
    canvas: &mut Canvas,
    font: &Font,
    text: &str,
    origin_x: f32,
    origin_y: f32,
    font_size_px: f32,
    color: Rgba,
) {
    if color.a == 0 || text.is_empty() {
        return;
    }

    let metrics = font.measure_text(text, font_size_px);
    let baseline_y = origin_y + metrics.ascent;
    let mut cursor_x = origin_x;

    for c in text.chars() {
        let glyph = font.rasterize_glyph(c, font_size_px);

        if glyph.width > 0 && glyph.height > 0 {
            let glyph_x0 = cursor_x + glyph.xmin as f32;
            let glyph_y0 = baseline_y - (glyph.ymin as f32 + glyph.height as f32);

            let gx_base = glyph_x0.round() as i64;
            let gy_base = glyph_y0.round() as i64;

            for gy in 0..glyph.height {
                let cy = gy_base + gy as i64;
                if cy < 0 || cy >= canvas.height as i64 {
                    continue;
                }

                let row_offset = gy * glyph.width;
                for gx in 0..glyph.width {
                    let cx = gx_base + gx as i64;
                    if cx < 0 || cx >= canvas.width as i64 {
                        continue;
                    }

                    let coverage = glyph.alpha_mask[row_offset + gx];
                    if coverage == 0 {
                        continue;
                    }

                    let effective_alpha =
                        ((color.a as f32) * (coverage as f32) / 255.0).round() as u8;
                    if effective_alpha > 0 {
                        let blended_color = Rgba::new(color.r, color.g, color.b, effective_alpha);
                        canvas.put_pixel(cx as u32, cy as u32, blended_color);
                    }
                }
            }
        }

        cursor_x += glyph.advance_width;
    }
}
