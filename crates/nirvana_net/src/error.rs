//! Network error definitions for resource loading.

use nirvana_url::UrlError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetError {
    InvalidUrl(UrlError),
    UnsupportedScheme(String),
    Io(String),
    HttpError(u16, String),
    NetworkFailure(String),
    DataUriDecodeError(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(err) => write!(f, "Invalid URL: {err}"),
            Self::UnsupportedScheme(s) => write!(f, "Unsupported scheme: {s}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::HttpError(code, msg) => write!(f, "HTTP {code}: {msg}"),
            Self::NetworkFailure(msg) => write!(f, "Network failure: {msg}"),
            Self::DataUriDecodeError(msg) => write!(f, "Data URI decode error: {msg}"),
        }
    }
}

impl std::error::Error for NetError {}

impl From<UrlError> for NetError {
    fn from(err: UrlError) -> Self {
        Self::InvalidUrl(err)
    }
}

impl From<std::io::Error> for NetError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
