//! CSS tokenizer implementation according to CSS Syntax Module Level 3.

use crate::token::{DimensionUnit, Token};

/// Tokenizer that converts raw CSS text into a stream of `Token`s.
pub struct CssTokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> CssTokenizer<'a> {
    /// Creates a new `CssTokenizer` for the given input.
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn peek_char_at(&self, n: usize) -> Option<char> {
        self.remaining().chars().nth(n)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.peek_char() {
            self.pos += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> bool {
        let mut saw_ws = false;
        loop {
            let rem = self.remaining();
            if rem.is_empty() {
                break;
            }

            if rem.starts_with("/*") {
                saw_ws = true;
                self.pos += 2;
                if let Some(end) = self.remaining().find("*/") {
                    self.pos += end + 2;
                } else {
                    self.pos = self.input.len();
                    break;
                }
                continue;
            }

            if let Some(ch) = self.peek_char()
                && ch.is_ascii_whitespace()
            {
                saw_ws = true;
                self.advance();
                continue;
            }

            break;
        }
        saw_ws
    }

    /// Consumes and returns the next CSS token along with whether whitespace preceded it.
    pub fn next_token_spanned(&mut self) -> (Token, bool) {
        loop {
            let had_ws = self.skip_whitespace_and_comments();

            let ch = match self.peek_char() {
                Some(c) => c,
                None => return (Token::Eof, had_ws),
            };

            // Number check: starts with digit, or '+' / '-' / '.' followed by digit
            if ch.is_ascii_digit()
                || ((ch == '+' || ch == '-')
                    && self
                        .peek_char_at(1)
                        .is_some_and(|c| c.is_ascii_digit() || (c == '.' && self.peek_char_at(2).is_some_and(|d| d.is_ascii_digit()))))
                || (ch == '.' && self.peek_char_at(1).is_some_and(|c| c.is_ascii_digit()))
            {
                return (self.consume_number_or_dimension(), had_ws);
            }

            match ch {
                '*' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        return (Token::SubstringMatch, had_ws);
                    }
                    return (Token::Star, had_ws);
                }
                '^' if self.peek_char_at(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    return (Token::PrefixMatch, had_ws);
                }
                '$' if self.peek_char_at(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    return (Token::SuffixMatch, had_ws);
                }
                '|' if self.peek_char_at(1) == Some('=') => {
                    self.advance();
                    self.advance();
                    return (Token::DashMatch, had_ws);
                }
                '~' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        return (Token::Includes, had_ws);
                    }
                    return (Token::Tilde, had_ws);
                }
                '{' => {
                    self.advance();
                    return (Token::OpenBrace, had_ws);
                }
                '}' => {
                    self.advance();
                    return (Token::CloseBrace, had_ws);
                }
                '(' => {
                    self.advance();
                    return (Token::OpenParen, had_ws);
                }
                ')' => {
                    self.advance();
                    return (Token::CloseParen, had_ws);
                }
                '[' => {
                    self.advance();
                    return (Token::OpenBracket, had_ws);
                }
                ']' => {
                    self.advance();
                    return (Token::CloseBracket, had_ws);
                }
                '>' => {
                    self.advance();
                    return (Token::Greater, had_ws);
                }
                '+' => {
                    self.advance();
                    return (Token::Plus, had_ws);
                }
                '!' => {
                    self.advance();
                    return (Token::Bang, had_ws);
                }
                '=' => {
                    self.advance();
                    return (Token::Equals, had_ws);
                }
                '/' => {
                    self.advance();
                    return (Token::Slash, had_ws);
                }
                ':' => {
                    self.advance();
                    return (Token::Colon, had_ws);
                }
                ';' => {
                    self.advance();
                    return (Token::Semicolon, had_ws);
                }
                ',' => {
                    self.advance();
                    return (Token::Comma, had_ws);
                }
                '.' => {
                    self.advance();
                    return (Token::Dot, had_ws);
                }
                '@' => {
                    self.advance();
                    let name = self.consume_ident_chars();
                    return (Token::AtKeyword(name), had_ws);
                }
                '#' => {
                    self.advance();
                    let name = self.consume_ident_chars();
                    return (Token::Hash(name), had_ws);
                }
                '"' | '\'' => {
                    let quote = self.advance().unwrap();
                    let mut s = String::new();
                    while let Some(c) = self.advance() {
                        if c == quote {
                            break;
                        }
                        if c == '\\'
                            && let Some(escaped) = self.advance()
                        {
                            s.push(escaped);
                            continue;
                        }
                        s.push(c);
                    }
                    return (Token::String(s), had_ws);
                }
                _ if is_ident_start(ch) => {
                    let ident = self.consume_ident_chars();
                    if self.peek_char() == Some('(') {
                        self.advance();
                        return (Token::Function(ident), had_ws);
                    }
                    return (Token::Ident(ident), had_ws);
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Consumes and returns the next CSS token.
    pub fn next_token(&mut self) -> Token {
        self.next_token_spanned().0
    }

    fn consume_ident_chars(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek_char() {
            if ch == '\\' {
                self.advance();
                if let Some(next) = self.peek_char() {
                    if next.is_ascii_hexdigit() {
                        let mut hex = String::new();
                        for _ in 0..6 {
                            if let Some(h) = self.peek_char()
                                && h.is_ascii_hexdigit()
                            {
                                hex.push(h);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        if self.peek_char().is_some_and(|w| w.is_ascii_whitespace()) {
                            self.advance();
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16)
                            && let Some(decoded) = char::from_u32(code)
                        {
                            s.push(decoded);
                        }
                    } else if next != '\n' {
                        s.push(next);
                        self.advance();
                    }
                }
            } else if is_ident_char(ch) {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn consume_number_or_dimension(&mut self) -> Token {
        let start = self.pos;
        if let Some(ch) = self.peek_char()
            && (ch == '+' || ch == '-')
        {
            self.advance();
        }

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        if let Some('.') = self.peek_char()
            && self.peek_char_at(1).is_some_and(|c| c.is_ascii_digit())
        {
            self.advance(); // consume '.'
            while let Some(ch) = self.peek_char() {
                if ch.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let num_str = &self.input[start..self.pos];
        let val: f32 = num_str.parse().unwrap_or(0.0);

        if let Some('%') = self.peek_char() {
            self.advance();
            return Token::Dimension(val, DimensionUnit::Percent);
        }

        if let Some(ch) = self.peek_char()
            && is_ident_start(ch)
        {
            let unit_str = self.consume_ident_chars();
            let unit = match unit_str.to_ascii_lowercase().as_str() {
                "px" => Some(DimensionUnit::Px),
                "em" => Some(DimensionUnit::Em),
                "rem" => Some(DimensionUnit::Rem),
                "pt" => Some(DimensionUnit::Pt),
                "vw" => Some(DimensionUnit::Vw),
                "vh" => Some(DimensionUnit::Vh),
                _ => None,
            };
            if let Some(u) = unit {
                return Token::Dimension(val, u);
            }
        }

        Token::Number(val)
    }

    /// Consumes all remaining tokens until `Token::Eof`.
    pub fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Consumes all tokens paired with whether leading whitespace preceded them.
    pub fn tokenize_all_spanned(&mut self) -> Vec<(Token, bool)> {
        let mut tokens = Vec::new();
        loop {
            let (tok, ws) = self.next_token_spanned();
            let is_eof = tok == Token::Eof;
            tokens.push((tok, ws));
            if is_eof {
                break;
            }
        }
        tokens
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || ch == '_'
        || ch == '-'
        || ch == '\\'
        || (!ch.is_ascii() && ch > '\u{0080}')
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || (!ch.is_ascii() && ch > '\u{0080}')
}
