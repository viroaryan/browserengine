//! HTML5 parser for Nirvana Browser Engine.
#![forbid(unsafe_code)]

pub mod token;

pub use token::{DoctypeData, TagData, Token};
