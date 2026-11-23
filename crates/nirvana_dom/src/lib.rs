//! DOM tree for Nirvana Browser Engine.
#![forbid(unsafe_code)]

pub mod node;
pub mod tree;

pub use node::{ElementData, Node, NodeData, NodeId, NodeType};
pub use tree::{DomError, DomTree};
