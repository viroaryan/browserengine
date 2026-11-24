//! CSS box model dimensions and geometry calculations.

use nirvana_core::geometry::Rect;

/// Edge sizes for margin, padding, and border.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeSizes {
    /// Creates a new `EdgeSizes`.
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// The four box model areas for a layout box:
/// - Content area (inner rectangle with width and height)
/// - Padding area
/// - Border area
/// - Margin area
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    /// Returns the padding box rectangle (content area expanded by padding).
    pub fn padding_box(&self) -> Rect {
        Rect::new(
            self.content.x() - self.padding.left,
            self.content.y() - self.padding.top,
            self.content.width() + self.padding.left + self.padding.right,
            self.content.height() + self.padding.top + self.padding.bottom,
        )
    }

    /// Returns the border box rectangle (padding box expanded by border).
    pub fn border_box(&self) -> Rect {
        let p_box = self.padding_box();
        Rect::new(
            p_box.x() - self.border.left,
            p_box.y() - self.border.top,
            p_box.width() + self.border.left + self.border.right,
            p_box.height() + self.border.top + self.border.bottom,
        )
    }

    /// Returns the margin box rectangle (border box expanded by margin).
    pub fn margin_box(&self) -> Rect {
        let b_box = self.border_box();
        Rect::new(
            b_box.x() - self.margin.left,
            b_box.y() - self.margin.top,
            b_box.width() + self.margin.left + self.margin.right,
            b_box.height() + self.margin.top + self.margin.bottom,
        )
    }
}
