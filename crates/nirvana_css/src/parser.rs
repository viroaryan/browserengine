//! CSS recursive-descent parser converting tokens into AST RuleSets and StyleSheets.

use crate::ast::{Declaration, PropertyValue, RuleSet, Selector, SimpleSelector, StyleSheet};
use crate::token::{DimensionUnit, Token};
use crate::tokenizer::CssTokenizer;
use nirvana_core::color::Rgba;

/// Parses a raw CSS string into a structured `StyleSheet`.
pub fn parse_css(input: &str) -> StyleSheet {
    let mut tokenizer = CssTokenizer::new(input);
    let tokens = tokenizer.tokenize_all();
    let mut parser = CssParser::new(tokens);
    parser.parse_stylesheet()
}

/// Parses inline CSS declarations directly (e.g. from an HTML `style` attribute).
pub fn parse_inline_declarations(input: &str) -> Vec<Declaration> {
    let mut tokenizer = CssTokenizer::new(input);
    let tokens = tokenizer.tokenize_all();
    let mut parser = CssParser::new(tokens);
    parser.parse_declarations()
}

struct CssParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl CssParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            Token::Eof
        }
    }

    fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut rules = Vec::new();
        while self.peek() != &Token::Eof {
            if let Some(rule) = self.parse_ruleset() {
                rules.push(rule);
            }
        }
        StyleSheet { rules }
    }

    fn parse_ruleset(&mut self) -> Option<RuleSet> {
        let selectors = self.parse_selectors();
        if selectors.is_empty() {
            // If no selectors found before EOF or brace, skip to next '{' or ';'
            while !matches!(self.peek(), Token::OpenBrace | Token::Eof) {
                self.advance();
            }
            if self.peek() == &Token::OpenBrace {
                self.advance();
                self.skip_until_block_end();
            }
            return None;
        }

        if self.peek() != &Token::OpenBrace {
            return None;
        }
        self.advance(); // consume '{'

        let declarations = self.parse_declarations();
        Some(RuleSet {
            selectors,
            declarations,
        })
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        let mut current_simple = Vec::new();

        while !matches!(self.peek(), Token::OpenBrace | Token::Semicolon | Token::Eof) {
            match self.advance() {
                Token::Star => {
                    current_simple.push(SimpleSelector::Universal);
                }
                Token::Ident(tag) => {
                    current_simple.push(SimpleSelector::Tag(tag.to_ascii_lowercase()));
                }
                Token::Dot => {
                    if let Token::Ident(cls) = self.peek() {
                        let cls = cls.clone();
                        self.advance();
                        current_simple.push(SimpleSelector::Class(cls));
                    }
                }
                Token::Hash(id) => {
                    current_simple.push(SimpleSelector::Id(id));
                }
                Token::Comma if !current_simple.is_empty() => {
                    selectors.push(Selector::new(std::mem::take(&mut current_simple)));
                }
                _ => {}
            }
        }

        if !current_simple.is_empty() {
            selectors.push(Selector::new(current_simple));
        }

        selectors
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut decls = Vec::new();

        while !matches!(self.peek(), Token::CloseBrace | Token::Eof) {
            if matches!(self.peek(), Token::Semicolon) {
                self.advance();
                continue;
            }

            let prop_name = match self.advance() {
                Token::Ident(name) => name,
                _ => {
                    self.skip_until_decl_end();
                    continue;
                }
            };

            if self.peek() != &Token::Colon {
                self.skip_until_decl_end();
                continue;
            }
            self.advance(); // consume ':'

            let mut value_tokens = Vec::new();
            while !matches!(self.peek(), Token::Semicolon | Token::CloseBrace | Token::Eof) {
                value_tokens.push(self.advance());
            }

            if self.peek() == &Token::Semicolon {
                self.advance();
            }

            self.process_declaration(&prop_name, &value_tokens, &mut decls);
        }

        if self.peek() == &Token::CloseBrace {
            self.advance();
        }

        decls
    }

    fn process_declaration(
        &self,
        prop: &str,
        tokens: &[Token],
        decls: &mut Vec<Declaration>,
    ) {
        let values: Vec<PropertyValue> = tokens
            .iter()
            .filter_map(Self::token_to_property_value)
            .collect();

        if values.is_empty() {
            return;
        }

        let prop_lower = prop.to_ascii_lowercase();

        // Shorthand expansion for margin and padding
        if prop_lower == "margin" || prop_lower == "padding" {
            let prefix = prop_lower.as_str();
            match values.len() {
                1 => {
                    let v = &values[0];
                    decls.push(Declaration {
                        name: format!("{prefix}-top"),
                        value: v.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-right"),
                        value: v.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-bottom"),
                        value: v.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-left"),
                        value: v.clone(),
                    });
                }
                2 => {
                    let v_tb = &values[0];
                    let v_lr = &values[1];
                    decls.push(Declaration {
                        name: format!("{prefix}-top"),
                        value: v_tb.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-right"),
                        value: v_lr.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-bottom"),
                        value: v_tb.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-left"),
                        value: v_lr.clone(),
                    });
                }
                3 => {
                    let v_t = &values[0];
                    let v_lr = &values[1];
                    let v_b = &values[2];
                    decls.push(Declaration {
                        name: format!("{prefix}-top"),
                        value: v_t.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-right"),
                        value: v_lr.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-bottom"),
                        value: v_b.clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-left"),
                        value: v_lr.clone(),
                    });
                }
                4 => {
                    decls.push(Declaration {
                        name: format!("{prefix}-top"),
                        value: values[0].clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-right"),
                        value: values[1].clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-bottom"),
                        value: values[2].clone(),
                    });
                    decls.push(Declaration {
                        name: format!("{prefix}-left"),
                        value: values[3].clone(),
                    });
                }
                _ => {}
            }
        } else if let Some(v) = values.into_iter().next() {
            decls.push(Declaration {
                name: prop_lower,
                value: v,
            });
        }
    }

    fn token_to_property_value(tok: &Token) -> Option<PropertyValue> {
        match tok {
            Token::Dimension(val, DimensionUnit::Px) => Some(PropertyValue::Px(*val)),
            Token::Number(val) if *val == 0.0 => Some(PropertyValue::Px(0.0)),
            Token::Number(_) => None,
            Token::Hash(hex) => {
                let formatted = format!("#{hex}");
                Rgba::from_hex(&formatted).ok().map(PropertyValue::Color)
            }
            Token::Ident(s) => {
                if s.eq_ignore_ascii_case("auto") {
                    Some(PropertyValue::Auto)
                } else if let Some(color) = parse_named_color(s) {
                    Some(PropertyValue::Color(color))
                } else {
                    Some(PropertyValue::Keyword(s.clone()))
                }
            }
            Token::String(s) => Some(PropertyValue::Keyword(s.clone())),
            _ => None,
        }
    }

    fn skip_until_decl_end(&mut self) {
        while !matches!(self.peek(), Token::Semicolon | Token::CloseBrace | Token::Eof) {
            self.advance();
        }
    }

    fn skip_until_block_end(&mut self) {
        let mut depth = 1;
        while depth > 0 && self.peek() != &Token::Eof {
            match self.advance() {
                Token::OpenBrace => depth += 1,
                Token::CloseBrace => depth -= 1,
                _ => {}
            }
        }
    }
}

fn parse_named_color(name: &str) -> Option<Rgba> {
    match name.to_ascii_lowercase().as_str() {
        "black" => Some(Rgba::new(0, 0, 0, 255)),
        "white" => Some(Rgba::new(255, 255, 255, 255)),
        "red" => Some(Rgba::new(255, 0, 0, 255)),
        "green" => Some(Rgba::new(0, 128, 0, 255)),
        "lime" => Some(Rgba::new(0, 255, 0, 255)),
        "blue" => Some(Rgba::new(0, 0, 255, 255)),
        "yellow" => Some(Rgba::new(255, 255, 0, 255)),
        "cyan" => Some(Rgba::new(0, 255, 255, 255)),
        "magenta" => Some(Rgba::new(255, 0, 255, 255)),
        "gray" | "grey" => Some(Rgba::new(128, 128, 128, 255)),
        "transparent" => Some(Rgba::new(0, 0, 0, 0)),
        _ => None,
    }
}
