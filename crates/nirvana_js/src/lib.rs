#![forbid(unsafe_code)]

pub mod console;
pub mod dom_binding;
pub mod runtime;

pub use console::{LogEntry, LogLevel};
pub use dom_binding::{DomHandle, SharedDom};
pub use runtime::JsRuntime;
