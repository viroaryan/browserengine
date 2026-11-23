//! WHATWG URL parser for Nirvana Browser Engine.
#![forbid(unsafe_code)]

pub mod error;
pub mod parser;

pub use error::UrlError;
pub use parser::Url;
