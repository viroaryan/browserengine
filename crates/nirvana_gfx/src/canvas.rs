//! 2D software pixel framebuffer with Porter-Duff alpha blending and rasterization.

use crate::display_list::{DisplayCommand, DisplayList};
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_layout::dimensions::EdgeSizes;

/// 32-bit RGBA software canvas/framebuffer.
/// Direct byte buffer layout: [R, G, B, A, R, G, B, A, ...] in row-major order.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    /// Creates a new `Canvas` with given dimensions initialized to transparent black (0, 0, 0, 0).
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0; size],
        }
    }

    /// Retrieves the pixel color at (x, y). Returns transparent black if out of bounds.
    pub fn pixel_at(&self, x: u32, y: u32) -> Rgba {
        if x >= self.width || y >= self.height {
            return Rgba::new(0, 0, 0, 0);
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        Rgba::new(
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        )
    }

    /// Sets a pixel with Porter-Duff 'Source Over' alpha blending.
    pub fn put_pixel(&mut self, x: u32, y: u32, color: Rgba) {
        if x >= self.width || y >= self.height || color.a == 0 {
            return;
        }

        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;

        if color.a == 255 {
            // Fast-path: fully opaque overwrite
            self.pixels[idx] = color.r;
            self.pixels[idx + 1] = color.g;
            self.pixels[idx + 2] = color.b;
            self.pixels[idx + 3] = 255;
            return;
        }

        // Porter-Duff 'Source Over' blending:
        let src_a = color.a as f32 / 255.0;
        let dst_a = self.pixels[idx + 3] as f32 / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);

        if out_a > 0.0 {
            let inv_src_a = 1.0 - src_a;
            let src_r = color.r as f32 * src_a;
            let src_g = color.g as f32 * src_a;
            let src_b = color.b as f32 * src_a;

            let dst_r = self.pixels[idx] as f32 * dst_a;
            let dst_g = self.pixels[idx + 1] as f32 * dst_a;
            let dst_b = self.pixels[idx + 2] as f32 * dst_a;

            let out_r = ((src_r + dst_r * inv_src_a) / out_a).round() as u8;
            let out_g = ((src_g + dst_g * inv_src_a) / out_a).round() as u8;
            let out_b = ((src_b + dst_b * inv_src_a) / out_a).round() as u8;

            self.pixels[idx] = out_r;
            self.pixels[idx + 1] = out_g;
            self.pixels[idx + 2] = out_b;
            self.pixels[idx + 3] = (out_a * 255.0).round() as u8;
        }
    }

    /// Fills an axis-aligned rectangle with a solid color, clamped to canvas boundaries.
    pub fn fill_rect(&mut self, rect: Rect, color: Rgba) {
        if color.a == 0 {
            return;
        }

        let x0 = (rect.x().floor() as i64).max(0).min(self.width as i64) as u32;
        let y0 = (rect.y().floor() as i64).max(0).min(self.height as i64) as u32;
        let x1 = ((rect.x() + rect.width()).ceil() as i64)
            .max(0)
            .min(self.width as i64) as u32;
        let y1 = ((rect.y() + rect.height()).ceil() as i64)
            .max(0)
            .min(self.height as i64) as u32;

        for y in y0..y1 {
            for x in x0..x1 {
                self.put_pixel(x, y, color);
            }
        }
    }

    /// Draws borders around a rectangle.
    pub fn draw_borders(&mut self, rect: Rect, widths: EdgeSizes, color: Rgba) {
        if color.a == 0 {
            return;
        }
        let rx = rect.x();
        let ry = rect.y();
        let rw = rect.width();
        let rh = rect.height();

        // Top border
        if widths.top > 0.0 {
            self.fill_rect(Rect::new(rx, ry, rw, widths.top), color);
        }
        // Bottom border
        if widths.bottom > 0.0 {
            self.fill_rect(
                Rect::new(rx, ry + rh - widths.bottom, rw, widths.bottom),
                color,
            );
        }
        // Left border
        if widths.left > 0.0 {
            self.fill_rect(Rect::new(rx, ry, widths.left, rh), color);
        }
        // Right border
        if widths.right > 0.0 {
            self.fill_rect(
                Rect::new(rx + rw - widths.right, ry, widths.right, rh),
                color,
            );
        }
    }

    /// Executes all commands in the display list onto this canvas.
    pub fn render_display_list(&mut self, list: &DisplayList) {
        for cmd in &list.commands {
            match cmd {
                DisplayCommand::SolidColor { rect, color } => {
                    self.fill_rect(*rect, *color);
                }
                DisplayCommand::Border {
                    rect,
                    widths,
                    color,
                } => {
                    self.draw_borders(*rect, *widths, *color);
                }
                DisplayCommand::Text { origin, color, .. } => {
                    // Primitive dot mark for text origin until Phase 6 font shaping
                    self.fill_rect(Rect::new(origin.x, origin.y, 4.0, 4.0), *color);
                }
            }
        }
    }
}
