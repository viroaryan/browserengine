//! WHATWG HTML5 streaming tokenizer state machine.

use crate::entities::decode_entities;
use crate::token::{DoctypeData, TagData, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    MarkupDeclarationOpen,
    Comment,
    Doctype,
}

pub struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
    state: State,
    current_tag: Option<TagData>,
    current_attr_name: String,
    current_attr_value: String,
    is_end_tag: bool,
    buffer: String,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        // Preprocess: CRLF / CR -> LF
        let mut chars = Vec::with_capacity(input.len());
        let mut prev_cr = false;
        for c in input.chars() {
            if c == '\r' {
                chars.push('\n');
                prev_cr = true;
            } else if c == '\n' {
                if !prev_cr {
                    chars.push('\n');
                }
                prev_cr = false;
            } else {
                chars.push(c);
                prev_cr = false;
            }
        }

        Self {
            chars,
            pos: 0,
            state: State::Data,
            current_tag: None,
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            is_end_tag: false,
            buffer: String::new(),
        }
    }

    #[inline]
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    #[inline]
    fn next_char(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    #[inline]
    fn starts_with_case_insensitive(&self, prefix: &str) -> bool {
        let prefix_chars: Vec<char> = prefix.chars().collect();
        if self.pos + prefix_chars.len() > self.chars.len() {
            return false;
        }
        for (i, &pc) in prefix_chars.iter().enumerate() {
            if !self.chars[self.pos + i].eq_ignore_ascii_case(&pc) {
                return false;
            }
        }
        true
    }

    fn commit_attribute(&mut self) {
        if !self.current_attr_name.is_empty() {
            let decoded_val = decode_entities(&self.current_attr_value);
            if let Some(tag) = &mut self.current_tag {
                tag.add_attribute(&self.current_attr_name, &decoded_val);
            }
            self.current_attr_name.clear();
            self.current_attr_value.clear();
        }
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            match self.state {
                State::Data => {
                    match self.next_char() {
                        Some('<') => {
                            self.state = State::TagOpen;
                        }
                        Some('&') => {
                            // Read entity
                            let mut entity_buf = String::from("&");
                            while let Some(c) = self.peek() {
                                if c == ';' {
                                    self.next_char();
                                    entity_buf.push(';');
                                    break;
                                } else if c.is_alphanumeric() || c == '#' {
                                    entity_buf.push(self.next_char().unwrap());
                                    if entity_buf.len() > 10 {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                            let decoded = decode_entities(&entity_buf);
                            for ch in decoded.chars() {
                                return Token::Character(ch);
                            }
                        }
                        Some(c) => {
                            return Token::Character(c);
                        }
                        None => {
                            return Token::Eof;
                        }
                    }
                }

                State::TagOpen => {
                    match self.peek() {
                        Some('!') => {
                            self.next_char();
                            self.state = State::MarkupDeclarationOpen;
                        }
                        Some('/') => {
                            self.next_char();
                            self.is_end_tag = true;
                            self.state = State::EndTagOpen;
                        }
                        Some('?') => {
                            // Bogus comment
                            self.next_char();
                            self.buffer.clear();
                            self.state = State::Comment;
                        }
                        Some(c) if c.is_ascii_alphabetic() => {
                            self.is_end_tag = false;
                            self.current_tag = Some(TagData::new("", false));
                            self.buffer.clear();
                            self.state = State::TagName;
                        }
                        Some(c) => {
                            self.state = State::Data;
                            return Token::Character(c);
                        }
                        None => {
                            self.state = State::Data;
                            return Token::Character('<');
                        }
                    }
                }

                State::EndTagOpen => {
                    match self.peek() {
                        Some(c) if c.is_ascii_alphabetic() => {
                            self.current_tag = Some(TagData::new("", false));
                            self.buffer.clear();
                            self.state = State::TagName;
                        }
                        Some('>') => {
                            self.next_char();
                            self.state = State::Data;
                        }
                        None => {
                            self.state = State::Data;
                            return Token::Character('<');
                        }
                        _ => {
                            // Bogus comment
                            self.buffer.clear();
                            self.state = State::Comment;
                        }
                    }
                }

                State::TagName => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {
                            if let Some(tag) = &mut self.current_tag {
                                tag.name = self.buffer.to_ascii_lowercase();
                            }
                            self.buffer.clear();
                            self.state = State::BeforeAttributeName;
                        }
                        Some('/') => {
                            if let Some(tag) = &mut self.current_tag {
                                tag.name = self.buffer.to_ascii_lowercase();
                            }
                            self.buffer.clear();
                            self.state = State::SelfClosingStartTag;
                        }
                        Some('>') => {
                            if let Some(tag) = &mut self.current_tag {
                                tag.name = self.buffer.to_ascii_lowercase();
                            }
                            self.buffer.clear();
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.buffer.push(c);
                        }
                        None => {
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::BeforeAttributeName => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {}
                        Some('/') => {
                            self.state = State::SelfClosingStartTag;
                        }
                        Some('>') => {
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.current_attr_name.clear();
                            self.current_attr_name.push(c);
                            self.state = State::AttributeName;
                        }
                        None => {
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::AttributeName => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {
                            self.state = State::AfterAttributeName;
                        }
                        Some('=') => {
                            self.state = State::BeforeAttributeValue;
                        }
                        Some('/') => {
                            self.commit_attribute();
                            self.state = State::SelfClosingStartTag;
                        }
                        Some('>') => {
                            self.commit_attribute();
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.current_attr_name.push(c);
                        }
                        None => {
                            self.commit_attribute();
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::AfterAttributeName => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {}
                        Some('=') => {
                            self.state = State::BeforeAttributeValue;
                        }
                        Some('/') => {
                            self.commit_attribute();
                            self.state = State::SelfClosingStartTag;
                        }
                        Some('>') => {
                            self.commit_attribute();
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.commit_attribute();
                            self.current_attr_name.push(c);
                            self.state = State::AttributeName;
                        }
                        None => {
                            self.commit_attribute();
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::BeforeAttributeValue => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {}
                        Some('"') => {
                            self.current_attr_value.clear();
                            self.state = State::AttributeValueDoubleQuoted;
                        }
                        Some('\'') => {
                            self.current_attr_value.clear();
                            self.state = State::AttributeValueSingleQuoted;
                        }
                        Some('>') => {
                            self.commit_attribute();
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.current_attr_value.clear();
                            self.current_attr_value.push(c);
                            self.state = State::AttributeValueUnquoted;
                        }
                        None => {
                            self.commit_attribute();
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::AttributeValueDoubleQuoted => {
                    match self.next_char() {
                        Some('"') => {
                            self.commit_attribute();
                            self.state = State::AfterAttributeValueQuoted;
                        }
                        Some(c) => {
                            self.current_attr_value.push(c);
                        }
                        None => {
                            self.commit_attribute();
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::AttributeValueSingleQuoted => {
                    match self.next_char() {
                        Some('\'') => {
                            self.commit_attribute();
                            self.state = State::AfterAttributeValueQuoted;
                        }
                        Some(c) => {
                            self.current_attr_value.push(c);
                        }
                        None => {
                            self.commit_attribute();
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::AttributeValueUnquoted => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {
                            self.commit_attribute();
                            self.state = State::BeforeAttributeName;
                        }
                        Some('>') => {
                            self.commit_attribute();
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.current_attr_value.push(c);
                        }
                        None => {
                            self.commit_attribute();
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::AfterAttributeValueQuoted => {
                    match self.next_char() {
                        Some('\t') | Some('\n') | Some(' ') => {
                            self.state = State::BeforeAttributeName;
                        }
                        Some('/') => {
                            self.state = State::SelfClosingStartTag;
                        }
                        Some('>') => {
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return if self.is_end_tag {
                                Token::EndTag(tag)
                            } else {
                                Token::StartTag(tag)
                            };
                        }
                        Some(c) => {
                            self.current_attr_name.clear();
                            self.current_attr_name.push(c);
                            self.state = State::AttributeName;
                        }
                        None => {
                            self.state = State::Data;
                            return Token::Eof;
                        }
                    }
                }

                State::SelfClosingStartTag => {
                    match self.next_char() {
                        Some('>') => {
                            if let Some(tag) = &mut self.current_tag {
                                tag.self_closing = true;
                            }
                            self.state = State::Data;
                            let tag = self.current_tag.take().unwrap();
                            return Token::StartTag(tag);
                        }
                        _ => {
                            self.state = State::BeforeAttributeName;
                        }
                    }
                }

                State::MarkupDeclarationOpen => {
                    if self.starts_with_case_insensitive("--") {
                        self.pos += 2;
                        self.buffer.clear();
                        self.state = State::Comment;
                    } else if self.starts_with_case_insensitive("DOCTYPE") {
                        self.pos += 7;
                        self.buffer.clear();
                        self.state = State::Doctype;
                    } else {
                        // Bogus comment
                        self.buffer.clear();
                        self.state = State::Comment;
                    }
                }

                State::Comment => {
                    let mut comment_content = String::new();
                    while let Some(c) = self.next_char() {
                        if c == '-' && self.peek() == Some('-') {
                            self.next_char(); // consume second -
                            if self.peek() == Some('>') {
                                self.next_char(); // consume >
                                self.state = State::Data;
                                return Token::Comment(comment_content);
                            } else {
                                comment_content.push('-');
                                comment_content.push('-');
                            }
                        } else {
                            comment_content.push(c);
                        }
                    }
                    self.state = State::Data;
                    return Token::Comment(comment_content);
                }

                State::Doctype => {
                    let mut doctype_str = String::new();
                    while let Some(c) = self.next_char() {
                        if c == '>' {
                            break;
                        }
                        doctype_str.push(c);
                    }
                    self.state = State::Data;
                    let name = doctype_str.split_whitespace().next().unwrap_or("html");
                    return Token::Doctype(DoctypeData::new(name));
                }
            }
        }
    }

    pub fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = matches!(tok, Token::Eof);
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }
}
