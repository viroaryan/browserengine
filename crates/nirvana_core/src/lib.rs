//! Core primitives for Nirvana Browser Engine.
#![forbid(unsafe_code)]

pub mod arena;
pub mod color;
pub mod geometry;

pub use arena::{Arena, Id};
pub use color::{ColorError, Rgba};
pub use geometry::{Point, Rect, Size};
