//! CSS token definitions according to CSS Syntax Module Level 3.

/// Recognized dimension units in CSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionUnit {
    Px,
    Em,
    Rem,
    Pt,
    Vw,
    Vh,
    Percent,
}

/// Token produced by the CSS tokenizer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Hash(String),
    String(String),
    Number(f32),
    Dimension(f32, DimensionUnit),
    AtKeyword(String),
    Function(String),
    Dot,
    Colon,
    Semicolon,
    Comma,
    Star,
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Greater,
    Plus,
    Tilde,
    Bang,
    Equals,
    Slash,
    Includes,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
    Eof,
}
