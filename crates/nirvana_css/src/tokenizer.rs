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

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let rem = self.remaining();
            if rem.is_empty() {
                break;
            }

            if rem.starts_with("/*") {
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
                self.advance();
                continue;
            }

            break;
        }
    }

    /// Consumes and returns the next CSS token.
    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_whitespace_and_comments();

            let ch = match self.peek_char() {
                Some(c) => c,
                None => return Token::Eof,
            };

            // Number check: starts with digit, or '+' / '-' / '.' followed by digit
            if ch.is_ascii_digit()
                || ((ch == '+' || ch == '-')
                    && self
                        .peek_char_at(1)
                        .is_some_and(|c| c.is_ascii_digit() || (c == '.' && self.peek_char_at(2).is_some_and(|d| d.is_ascii_digit()))))
                || (ch == '.' && self.peek_char_at(1).is_some_and(|c| c.is_ascii_digit()))
            {
                return self.consume_number_or_dimension();
            }

            match ch {
                '*' => {
                    self.advance();
                    return Token::Star;
                }
                '{' => {
                    self.advance();
                    return Token::OpenBrace;
                }
                '}' => {
                    self.advance();
                    return Token::CloseBrace;
                }
                ':' => {
                    self.advance();
                    return Token::Colon;
                }
                ';' => {
                    self.advance();
                    return Token::Semicolon;
                }
                ',' => {
                    self.advance();
                    return Token::Comma;
                }
                '.' => {
                    self.advance();
                    return Token::Dot;
                }
                '#' => {
                    self.advance();
                    let name = self.consume_ident_chars();
                    return Token::Hash(name);
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
                    return Token::String(s);
                }
                _ if is_ident_start(ch) => {
                    let ident = self.consume_ident_chars();
                    return Token::Ident(ident);
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn consume_ident_chars(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek_char() {
            if is_ident_char(ch) {
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
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '-' || (!ch.is_ascii() && ch > '\u{0080}')
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || (!ch.is_ascii() && ch > '\u{0080}')
}
