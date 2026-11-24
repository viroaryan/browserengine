//! Typography metrics calculation and text measurement.

use crate::font::Font;

/// Dimensions and line alignment metrics for a measured text string.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMetrics {
    /// Total horizontal advance width of the text in CSS pixels.
    pub width: f32,
    /// Distance from highest ascent to lowest descent (ascent - descent).
    pub height: f32,
    /// Distance from baseline to the highest point of glyphs (positive).
    pub ascent: f32,
    /// Distance from baseline to the lowest point of glyphs (negative).
    pub descent: f32,
    /// Standard vertical line advance including line gap.
    pub line_height: f32,
}

impl Font {
    /// Measures the dimensions of a text string at the specified font size in CSS pixels.
    pub fn measure_text(&self, text: &str, font_size_px: f32) -> TextMetrics {
        let (ascent, descent, _line_gap, line_height) =
            if let Some(lm) = self.inner().horizontal_line_metrics(font_size_px) {
                let asc = lm.ascent;
                let desc = lm.descent;
                let gap = lm.line_gap;
                let lh = lm.new_line_size;
                (asc, desc, gap, lh)
            } else {
                let asc = font_size_px * 0.8;
                let desc = -font_size_px * 0.2;
                let gap = font_size_px * 0.2;
                let lh = font_size_px * 1.2;
                (asc, desc, gap, lh)
            };

        let mut total_width: f32 = 0.0;
        for c in text.chars() {
            let metrics = self.inner().metrics(c, font_size_px);
            total_width += metrics.advance_width;
        }

        let height = (ascent - descent).max(font_size_px);

        TextMetrics {
            width: total_width,
            height,
            ascent,
            descent,
            line_height: line_height.max(height),
        }
    }

    /// Returns the recommended vertical line height for this font at the given size.
    pub fn line_height(&self, font_size_px: f32) -> f32 {
        self.measure_text("", font_size_px).line_height
    }
}
