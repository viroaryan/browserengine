//! HTML5 parser for Nirvana Browser Engine.
#![forbid(unsafe_code)]

pub mod entities;
pub mod token;
pub mod tokenizer;

pub use token::{DoctypeData, TagData, Token};
pub use tokenizer::Tokenizer;
