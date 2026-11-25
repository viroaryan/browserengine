//! Computed style value representations.

use nirvana_core::color::Rgba;

/// Display layout mode for an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Inline,
    Block,
    InlineBlock,
    Flex,
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
    ListItem,
    None,
}

/// CSS positioning scheme for layout flow handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

/// Text horizontal alignment mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Dimension or length property value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

/// Fully computed and resolved style properties for a DOM element.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedValues {
    pub display: Display,
    pub position: Position,
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
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,
    pub border_color: Option<Rgba>,
    pub border_radius: f32,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub max_width: Length,
    pub min_height: Length,
    pub font_size: f32,
    pub font_weight: u16,
    pub font_style_italic: bool,
    pub text_decoration_underline: bool,
    pub text_align: TextAlign,
    pub line_height: f32,
    pub flex_direction_column: bool,
    pub flex_wrap: bool,
    pub is_grid: bool,
    pub grid_cols: usize,
    pub overflow_hidden: bool,
    pub gap: f32,
    pub visibility_hidden: bool,
    pub offscreen_hidden: bool,
}

impl Default for ComputedValues {
    fn default() -> Self {
        Self {
            display: Display::Inline,
            position: Position::Static,
            color: Rgba::new(0, 0, 0, 255),
            background_color: Rgba::new(0, 0, 0, 0),
            margin_top: Length::Px(0.0),
            margin_right: Length::Px(0.0),
            margin_bottom: Length::Px(0.0),
            margin_left: Length::Px(0.0),
            padding_top: Length::Px(0.0),
            padding_right: Length::Px(0.0),
            padding_bottom: Length::Px(0.0),
            padding_left: Length::Px(0.0),
            border_width: 0.0,
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            border_color: None,
            border_radius: 0.0,
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            max_width: Length::Auto,
            min_height: Length::Auto,
            font_size: 16.0,
            font_weight: 400,
            font_style_italic: false,
            text_decoration_underline: false,
            text_align: TextAlign::Left,
            line_height: 0.0,
            flex_direction_column: false,
            flex_wrap: false,
            is_grid: false,
            grid_cols: 1,
            overflow_hidden: false,
            gap: 0.0,
            visibility_hidden: false,
            offscreen_hidden: false,
        }
    }
}
