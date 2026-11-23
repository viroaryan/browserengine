//! HTML5 parser for Nirvana Browser Engine.
#![forbid(unsafe_code)]

pub mod entities;
pub mod token;
pub mod tokenizer;
pub mod tree_builder;

pub use token::{DoctypeData, TagData, Token};
pub use tokenizer::Tokenizer;
pub use tree_builder::TreeBuilder;

use nirvana_dom::DomTree;

/// Parses an HTML string into an arena-backed DOM tree.
pub fn parse_html(html: &str) -> DomTree {
    let tokens = Tokenizer::new(html).tokenize_all();
    TreeBuilder::new(tokens).build()
}
