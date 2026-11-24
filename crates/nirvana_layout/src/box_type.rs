//! Layout box types and node representations.

use crate::dimensions::Dimensions;
use nirvana_core::arena::Id;
use nirvana_dom::node::Node;

/// The formatting type of a layout box.
#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    /// A block-level container backed by a DOM node (e.g. `div`, `p`, `html`, `body`).
    BlockNode(Id<Node>),
    /// An inline-level box backed by a DOM node (e.g. `span`, `a`, text node).
    InlineNode(Id<Node>),
    /// An anonymous block box generated to wrap inline children inside a block container
    /// that also contains block-level children (CSS 2.1 §9.2.1.1).
    AnonymousBlock,
}

/// A node in the Layout Tree representing a formatted box with box-model dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    pub box_type: BoxType,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,
}

impl LayoutBox {
    /// Creates a new `LayoutBox` with default dimensions and no children.
    pub fn new(box_type: BoxType) -> Self {
        Self {
            box_type,
            dimensions: Dimensions::default(),
            children: Vec::new(),
        }
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
}
