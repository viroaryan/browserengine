//! Computed style value representations.

use nirvana_core::color::Rgba;

/// Display layout mode for an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Inline,
    Block,
    None,
}

/// Dimension or length property value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
}

/// Fully computed and resolved style properties for a DOM element.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedValues {
    pub display: Display,
    pub color: Rgba,
    pub background_color: Rgba,
    pub margin_top: Length,
    pub margin_right: Length,
    pub margin_bottom: Length,
    pub margin_left: Length,
    pub padding_top: Length,
    pub padding_right: Length,
    pub padding_bottom: Length,
    pub padding_left: Length,
    pub border_width: f32,
    pub width: Length,
    pub height: Length,
}

impl Default for ComputedValues {
    fn default() -> Self {
        Self {
            display: Display::Inline,
            color: Rgba::new(0, 0, 0, 255), // Default text color: black
            background_color: Rgba::new(0, 0, 0, 0), // Transparent
            margin_top: Length::Px(0.0),
            margin_right: Length::Px(0.0),
            margin_bottom: Length::Px(0.0),
            margin_left: Length::Px(0.0),
            padding_top: Length::Px(0.0),
            padding_right: Length::Px(0.0),
            padding_bottom: Length::Px(0.0),
            padding_left: Length::Px(0.0),
            border_width: 0.0,
            width: Length::Auto,
            height: Length::Auto,
        }
    }
}
