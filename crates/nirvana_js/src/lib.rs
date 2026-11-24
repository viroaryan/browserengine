#![forbid(unsafe_code)]

pub mod console;
pub mod runtime;

pub use console::{LogEntry, LogLevel};
pub use runtime::JsRuntime;
