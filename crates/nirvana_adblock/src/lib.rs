//! Sovereign, pure-Rust ad-blocking and content filtering engine modeled on uBlock Origin.
//! Provides network request interception, cosmetic element hiding, and scriptlet defusers.

#![forbid(unsafe_code)]

pub mod cosmetic;
pub mod engine;
pub mod filter;
pub mod matcher;
pub mod parser;
pub mod rulesets;
pub mod scriptlets;

pub use engine::AdBlockEngine;
pub use filter::{ContentType, CosmeticRule, FilterAction, FilterRule, NetworkRule};
pub use parser::{parse_filter_list, parse_line};
