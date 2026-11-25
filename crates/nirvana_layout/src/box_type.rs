//! Layout box types and node representations.

use crate::dimensions::Dimensions;
use nirvana_core::arena::Id;
use nirvana_core::geometry::{Rect, Size};
use nirvana_dom::node::Node;
use nirvana_style::values::{Display, Position};

/// A single shaped, positioned line of text produced during inline flow layout.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub rect: Rect,
    pub font_size: f32,
}

/// The formatting type of a layout box.
#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    /// A block-level container backed by a DOM node (e.g. `div`, `p`, `html`, `body`, `table`, `flex`).
    BlockNode(Id<Node>),
    /// An inline-level box backed by a DOM node (e.g. `span`, `a`, text node, `img`, `inline-block`).
    InlineNode(Id<Node>),
    /// An anonymous block box generated to wrap inline children inside a block container
    /// that also contains block-level children (CSS 2.1 §9.2.1.1).
    AnonymousBlock,
}

/// A node in the Layout Tree representing a formatted box with box-model dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    pub box_type: BoxType,
    pub display_mode: Display,
    pub position_mode: Position,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,
    /// Intrinsic dimensions for replaced elements (e.g. `<img>`, `<input>`, `<button>`).
    pub intrinsic_size: Option<Size>,
    /// Raw collapsed text for `NodeType::Text` boxes or `<select>` selected text.
    pub text_content: Option<String>,
    /// Laid-out per-line text runs produced by inline flow layout.
    pub text_runs: Vec<TextRun>,
    /// True when this box represents a forced line break (`<br>`).
    pub is_line_break: bool,
}

impl LayoutBox {
    /// Creates a new `LayoutBox` with default dimensions and no children.
    pub fn new(box_type: BoxType) -> Self {
        let display_mode = match box_type {
            BoxType::BlockNode(_) | BoxType::AnonymousBlock => Display::Block,
            BoxType::InlineNode(_) => Display::Inline,
        };
        Self {
            box_type,
            display_mode,
            position_mode: Position::Static,
            dimensions: Dimensions::default(),
            children: Vec::new(),
            intrinsic_size: None,
            text_content: None,
            text_runs: Vec::new(),
            is_line_break: false,
        }
    }

    /// Returns `true` if this box is out of normal document flow (`absolute` or `fixed`).
    #[inline]
    pub fn is_out_of_flow(&self) -> bool {
        matches!(self.position_mode, Position::Absolute | Position::Fixed)
    }

    /// Returns `true` if this box represents a replaced element (like `<img>`).
    #[inline]
    pub fn is_replaced(&self) -> bool {
        self.intrinsic_size.is_some()
    }

    /// Returns the DOM node ID associated with this box, if any.
    pub fn node_id(&self) -> Option<Id<Node>> {
        match self.box_type {
            BoxType::BlockNode(id) | BoxType::InlineNode(id) => Some(id),
            BoxType::AnonymousBlock => None,
        }
    }

    /// Returns `true` if this is a block-level box (`BlockNode` or `AnonymousBlock`).
    pub fn is_block_level(&self) -> bool {
        matches!(self.box_type, BoxType::BlockNode(_) | BoxType::AnonymousBlock)
    }

    /// Returns `true` if this is an inline-level box (`InlineNode`).
    pub fn is_inline_level(&self) -> bool {
        matches!(self.box_type, BoxType::InlineNode(_))
    }

    /// Translates this box, its laid-out text runs, and all descendants by `(dx, dy)`.
    pub fn shift_by(&mut self, dx: f32, dy: f32) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.dimensions.content.origin.x += dx;
        self.dimensions.content.origin.y += dy;
        for run in &mut self.text_runs {
            run.rect.origin.x += dx;
            run.rect.origin.y += dy;
        }
        for child in &mut self.children {
            child.shift_by(dx, dy);
        }
    }
}
