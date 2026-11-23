//! URL parsing errors.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    EmptyInput,
    MissingScheme,
    InvalidScheme,
    InvalidPort,
    InvalidHost,
}

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "Empty URL string"),
            Self::MissingScheme => write!(f, "Missing URL scheme"),
            Self::InvalidScheme => write!(f, "Invalid URL scheme format"),
            Self::InvalidPort => write!(f, "Invalid URL port"),
            Self::InvalidHost => write!(f, "Invalid URL host"),
        }
    }
}

impl std::error::Error for UrlError {}
