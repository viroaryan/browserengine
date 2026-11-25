//! CSS recursive-descent parser converting tokens into AST RuleSets and StyleSheets.

use crate::ast::{
    AttrOp, Combinator, Declaration, PropertyValue, RuleSet, Selector, SelectorSegment,
    SimpleSelector, StyleSheet,
};
use crate::token::{DimensionUnit, Token};
use crate::tokenizer::CssTokenizer;
use nirvana_core::color::Rgba;

const DEFAULT_MEDIA_VIEWPORT_WIDTH: f32 = 392.0;

/// Parses a raw CSS string into a structured `StyleSheet`.
pub fn parse_css(input: &str) -> StyleSheet {
    let mut tokenizer = CssTokenizer::new(input);
    let tokens = tokenizer.tokenize_all_spanned();
    let mut parser = CssParser::new(tokens);
    parser.parse_stylesheet()
}

/// Parses inline CSS declarations directly (e.g. from an HTML `style` attribute).
pub fn parse_inline_declarations(input: &str) -> Vec<Declaration> {
    let mut tokenizer = CssTokenizer::new(input);
    let tokens = tokenizer.tokenize_all_spanned();
    let mut parser = CssParser::new(tokens);
    parser.parse_declarations()
}

struct CssParser {
    tokens: Vec<(Token, bool)>,
    pos: usize,
}

impl CssParser {
    fn new(tokens: Vec<(Token, bool)>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|(t, _)| t)
            .unwrap_or(&Token::Eof)
    }

    fn peek_spanned(&self) -> (&Token, bool) {
        self.tokens
            .get(self.pos)
            .map(|(t, ws)| (t, *ws))
            .unwrap_or((&Token::Eof, false))
    }

    fn advance_spanned(&mut self) -> (Token, bool) {
        if self.pos < self.tokens.len() {
            let item = self.tokens[self.pos].clone();
            self.pos += 1;
            item
        } else {
            (Token::Eof, false)
        }
    }

    fn advance(&mut self) -> Token {
        self.advance_spanned().0
    }

    fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut rules = Vec::new();
        self.parse_rules_until(false, &mut rules);
        StyleSheet { rules }
    }

    fn parse_rules_until(&mut self, stop_on_close_brace: bool, rules: &mut Vec<RuleSet>) {
        while self.peek() != &Token::Eof {
            if self.peek() == &Token::CloseBrace {
                self.advance();
                if stop_on_close_brace {
                    break;
                }
                continue;
            }

            if let Token::AtKeyword(kw) = self.peek().clone() {
                self.parse_at_rule(&kw, rules);
                continue;
            }

            if let Some(rule) = self.parse_ruleset() {
                rules.push(rule);
            }
        }
    }

    fn parse_at_rule(&mut self, kw: &str, rules: &mut Vec<RuleSet>) {
        self.advance(); // consume @keyword
        let kw_lower = kw.to_ascii_lowercase();

        let mut header_tokens = Vec::new();
        while !matches!(self.peek(), Token::OpenBrace | Token::Semicolon | Token::Eof) {
            header_tokens.push(self.advance());
        }

        if self.peek() == &Token::Semicolon {
            self.advance();
            return;
        }
        if self.peek() != &Token::OpenBrace {
            return;
        }
        self.advance(); // consume '{'

        if kw_lower == "media" && media_query_matches(&header_tokens) {
            self.parse_rules_until(true, rules);
        } else if matches!(kw_lower.as_str(), "supports" | "layer") {
            self.parse_rules_until(true, rules);
        } else {
            self.skip_until_block_end();
        }
    }

    fn parse_ruleset(&mut self) -> Option<RuleSet> {
        let selectors = self.parse_selectors();
        if selectors.is_empty() {
            while !matches!(
                self.peek(),
                Token::OpenBrace | Token::CloseBrace | Token::Semicolon | Token::Eof
            ) {
                self.advance();
            }
            match self.peek() {
                Token::OpenBrace => {
                    self.advance();
                    self.skip_until_block_end();
                }
                Token::Semicolon => {
                    self.advance();
                }
                _ => {}
            }
            return None;
        }

        if self.peek() != &Token::OpenBrace {
            if self.peek() == &Token::Semicolon {
                self.advance();
            }
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
        let mut segments: Vec<Vec<SimpleSelector>> = Vec::new();
        let mut combinators: Vec<Combinator> = Vec::new();
        let mut current_compound: Vec<SimpleSelector> = Vec::new();
        let mut pending_combinator: Option<Combinator> = None;

        while !matches!(
            self.peek(),
            Token::OpenBrace | Token::CloseBrace | Token::Semicolon | Token::Eof
        ) {
            let (tok, had_ws) = self.peek_spanned();
            if matches!(tok, Token::Comma) {
                self.advance();
                push_built_selector(
                    &mut selectors,
                    &mut segments,
                    &mut combinators,
                    &mut current_compound,
                );
                pending_combinator = None;
                continue;
            }

            if matches!(tok, Token::Greater | Token::Plus | Token::Tilde) {
                let comb = match tok {
                    Token::Greater => Combinator::Child,
                    Token::Plus => Combinator::NextSibling,
                    _ => Combinator::SubsequentSibling,
                };
                self.advance();
                if !current_compound.is_empty() {
                    if let Some(prev_comb) = pending_combinator.take()
                        && !segments.is_empty()
                    {
                        combinators.push(prev_comb);
                    }
                    segments.push(std::mem::take(&mut current_compound));
                    pending_combinator = Some(comb);
                }
                continue;
            }

            // Leading whitespace starts a new segment with Descendant combinator
            if had_ws && !current_compound.is_empty() {
                if let Some(prev_comb) = pending_combinator.take()
                    && !segments.is_empty()
                {
                    combinators.push(prev_comb);
                }
                segments.push(std::mem::take(&mut current_compound));
                pending_combinator = Some(Combinator::Descendant);
            }

            match self.advance() {
                Token::Star => current_compound.push(SimpleSelector::Universal),
                Token::Ident(tag) => {
                    current_compound.push(SimpleSelector::Tag(tag.to_ascii_lowercase()));
                }
                Token::Dot => {
                    if let Token::Ident(cls) = self.peek() {
                        let cls = cls.clone();
                        self.advance();
                        current_compound.push(SimpleSelector::Class(cls));
                    }
                }
                Token::Hash(id) => current_compound.push(SimpleSelector::Id(id)),
                Token::OpenBracket => {
                    if let Some(attr_sel) = self.parse_attribute_selector() {
                        current_compound.push(attr_sel);
                    }
                }
                Token::Colon => {
                    let mut is_pseudo_elem = false;
                    if self.peek() == &Token::Colon {
                        self.advance();
                        is_pseudo_elem = true;
                    }
                    match self.peek().clone() {
                        Token::Ident(name) => {
                            self.advance();
                            let prefix = if is_pseudo_elem { ":" } else { "" };
                            current_compound.push(SimpleSelector::PseudoClass(format!(
                                "{prefix}{}",
                                name.to_ascii_lowercase()
                            )));
                        }
                        Token::Function(fname) => {
                            self.advance();
                            self.skip_until_paren_end();
                            let prefix = if is_pseudo_elem { ":" } else { "" };
                            current_compound.push(SimpleSelector::PseudoClass(format!(
                                "{prefix}{}()",
                                fname.to_ascii_lowercase()
                            )));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some(prev_comb) = pending_combinator
            && !current_compound.is_empty()
            && !segments.is_empty()
        {
            combinators.push(prev_comb);
        }
        push_built_selector(
            &mut selectors,
            &mut segments,
            &mut combinators,
            &mut current_compound,
        );

        selectors
    }

    fn parse_attribute_selector(&mut self) -> Option<SimpleSelector> {
        let name = match self.peek().clone() {
            Token::Ident(n) => {
                self.advance();
                n.to_ascii_lowercase()
            }
            _ => {
                while !matches!(self.peek(), Token::CloseBracket | Token::Eof) {
                    self.advance();
                }
                if self.peek() == &Token::CloseBracket {
                    self.advance();
                }
                return None;
            }
        };

        let op = match self.peek() {
            Token::CloseBracket => {
                self.advance();
                return Some(SimpleSelector::Attribute {
                    name,
                    op: AttrOp::Exists,
                    value: None,
                });
            }
            Token::Equals => {
                self.advance();
                AttrOp::Exact
            }
            Token::Includes => {
                self.advance();
                AttrOp::Includes
            }
            Token::DashMatch => {
                self.advance();
                AttrOp::DashMatch
            }
            Token::PrefixMatch => {
                self.advance();
                AttrOp::Prefix
            }
            Token::SuffixMatch => {
                self.advance();
                AttrOp::Suffix
            }
            Token::SubstringMatch => {
                self.advance();
                AttrOp::Contains
            }
            _ => {
                while !matches!(self.peek(), Token::CloseBracket | Token::Eof) {
                    self.advance();
                }
                if self.peek() == &Token::CloseBracket {
                    self.advance();
                }
                return None;
            }
        };

        let value = match self.peek().clone() {
            Token::String(s) | Token::Ident(s) => {
                self.advance();
                Some(s)
            }
            Token::Number(n) => {
                self.advance();
                Some(n.to_string())
            }
            _ => None,
        };

        while !matches!(self.peek(), Token::CloseBracket | Token::Eof) {
            self.advance();
        }
        if self.peek() == &Token::CloseBracket {
            self.advance();
        }

        Some(SimpleSelector::Attribute { name, op, value })
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
        let (clean_tokens, important) = strip_important(tokens);
        let values = parse_value_list(clean_tokens);
        if values.is_empty() {
            return;
        }

        let prop_lower = prop.to_ascii_lowercase();
        let push = |name: &str, value: PropertyValue, out: &mut Vec<Declaration>| {
            out.push(Declaration::with_importance(name, value, important));
        };

        // 1. Margin and Padding shorthands (1..4 values)
        if prop_lower == "margin" || prop_lower == "padding" {
            let prefix = prop_lower.as_str();
            let valid_vals: Vec<&PropertyValue> = values
                .iter()
                .filter(|v| is_valid_length_or_auto(v))
                .collect();
            match valid_vals.len() {
                1 => {
                    let v = valid_vals[0];
                    push(&format!("{prefix}-top"), v.clone(), decls);
                    push(&format!("{prefix}-right"), v.clone(), decls);
                    push(&format!("{prefix}-bottom"), v.clone(), decls);
                    push(&format!("{prefix}-left"), v.clone(), decls);
                }
                2 => {
                    let (v_tb, v_lr) = (valid_vals[0], valid_vals[1]);
                    push(&format!("{prefix}-top"), v_tb.clone(), decls);
                    push(&format!("{prefix}-right"), v_lr.clone(), decls);
                    push(&format!("{prefix}-bottom"), v_tb.clone(), decls);
                    push(&format!("{prefix}-left"), v_lr.clone(), decls);
                }
                3 => {
                    let (v_t, v_lr, v_b) = (valid_vals[0], valid_vals[1], valid_vals[2]);
                    push(&format!("{prefix}-top"), v_t.clone(), decls);
                    push(&format!("{prefix}-right"), v_lr.clone(), decls);
                    push(&format!("{prefix}-bottom"), v_b.clone(), decls);
                    push(&format!("{prefix}-left"), v_lr.clone(), decls);
                }
                4 => {
                    push(&format!("{prefix}-top"), valid_vals[0].clone(), decls);
                    push(&format!("{prefix}-right"), valid_vals[1].clone(), decls);
                    push(&format!("{prefix}-bottom"), valid_vals[2].clone(), decls);
                    push(&format!("{prefix}-left"), valid_vals[3].clone(), decls);
                }
                _ => {}
            }
            return;
        }

        // 2. Background shorthand (background: #f1f5f9 ... -> background-color)
        if prop_lower == "background" {
            if let Some(color) = values.iter().find_map(|v| match v {
                PropertyValue::Color(c) => Some(*c),
                _ => None,
            }) {
                push("background-color", PropertyValue::Color(color), decls);
            }
            return;
        }

        // 3. Border shorthands (border, border-top, border-right, border-bottom, border-left)
        if matches!(
            prop_lower.as_str(),
            "border" | "border-top" | "border-right" | "border-bottom" | "border-left"
        ) {
            let is_none = values.iter().any(|v| {
                matches!(v, PropertyValue::Keyword(k) if k.eq_ignore_ascii_case("none"))
            });
            let width_prop = if prop_lower == "border" {
                "border-width".to_string()
            } else {
                format!("{prop_lower}-width")
            };

            if is_none {
                push(&width_prop, PropertyValue::Px(0.0), decls);
                return;
            }
            if let Some(w) = values.iter().find(|v| is_valid_length_or_auto(v)) {
                push(&width_prop, (*w).clone(), decls);
            }
            if let Some(PropertyValue::Color(c)) = values
                .iter()
                .find(|v| matches!(v, PropertyValue::Color(_)))
            {
                push("border-color", PropertyValue::Color(*c), decls);
            }
            return;
        }

        // 4. Font shorthand (e.g. font: bold 14px/1.4 Arial)
        if prop_lower == "font" {
            for v in &values {
                match v {
                    PropertyValue::Keyword(k) if k.eq_ignore_ascii_case("bold") => {
                        push("font-weight", v.clone(), decls);
                    }
                    PropertyValue::Keyword(k) if k.eq_ignore_ascii_case("italic") => {
                        push("font-style", v.clone(), decls);
                    }
                    v if is_valid_length_or_auto(v) && !matches!(v, PropertyValue::Auto) => {
                        push("font-size", v.clone(), decls);
                    }
                    _ => {}
                }
            }
            return;
        }

        // 4.5. Flex-flow shorthand (e.g. flex-flow: column nowrap | row wrap)
        if prop_lower == "flex-flow" {
            for v in &values {
                if let PropertyValue::Keyword(k) = v {
                    let low = k.to_ascii_lowercase();
                    match low.as_str() {
                        "row" | "row-reverse" | "column" | "column-reverse" => {
                            push("flex-direction", PropertyValue::Keyword(low), decls);
                        }
                        "wrap" | "wrap-reverse" | "nowrap" => {
                            push("flex-wrap", PropertyValue::Keyword(low), decls);
                        }
                        _ => {}
                    }
                }
            }
            return;
        }

        // 4.6. Grid-template-columns multi-track summary
        if prop_lower == "grid-template-columns" {
            let summary = summarize_grid_template_columns(clean_tokens, &values);
            push("grid-template-columns", PropertyValue::Keyword(summary), decls);
            return;
        }

        // 5. Strict CSS length property filter (rejects nonzero unitless numbers for dimensions)
        if is_length_property(&prop_lower) {
            if let Some(v) = values.into_iter().find(is_valid_length_or_auto) {
                push(&prop_lower, v, decls);
            }
            return;
        }

        if let Some(v) = values.into_iter().next() {
            push(&prop_lower, v, decls);
        }
    }

    fn skip_until_decl_end(&mut self) {
        let mut brace_depth = 0usize;
        while self.peek() != &Token::Eof {
            match self.peek() {
                Token::OpenBrace => {
                    brace_depth += 1;
                    self.advance();
                }
                Token::CloseBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                    self.advance();
                    if brace_depth == 0 {
                        break;
                    }
                }
                Token::Semicolon if brace_depth == 0 => {
                    self.advance();
                    break;
                }
                _ => {
                    self.advance();
                }
            }
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

    fn skip_until_paren_end(&mut self) {
        let mut depth = 1;
        while depth > 0 && self.peek() != &Token::Eof {
            match self.advance() {
                Token::Function(_) | Token::OpenParen => depth += 1,
                Token::CloseParen => depth -= 1,
                _ => {}
            }
        }
    }
}

fn push_built_selector(
    selectors: &mut Vec<Selector>,
    segments: &mut Vec<Vec<SimpleSelector>>,
    combinators: &mut Vec<Combinator>,
    current_compound: &mut Vec<SimpleSelector>,
) {
    if !current_compound.is_empty() {
        segments.push(std::mem::take(current_compound));
    }
    if segments.is_empty() {
        combinators.clear();
        return;
    }

    let subject = segments.pop().unwrap();
    let mut ancestors = Vec::new();
    while let Some(seg_simples) = segments.pop() {
        let comb = combinators.pop().unwrap_or(Combinator::Descendant);
        ancestors.push(SelectorSegment {
            combinator: comb,
            simple_selectors: seg_simples,
        });
    }
    combinators.clear();
    selectors.push(Selector::with_ancestors(subject, ancestors));
}

fn strip_important(tokens: &[Token]) -> (&[Token], bool) {
    if tokens.len() >= 2
        && matches!(tokens[tokens.len() - 2], Token::Bang)
        && matches!(&tokens[tokens.len() - 1], Token::Ident(s) if s.eq_ignore_ascii_case("important"))
    {
        return (&tokens[..tokens.len() - 2], true);
    }
    (tokens, false)
}

fn parse_value_list(tokens: &[Token]) -> Vec<PropertyValue> {
    let mut values = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Function(fname) => {
                let fn_lower = fname.to_ascii_lowercase();
                let mut depth = 1usize;
                let mut inner = Vec::new();
                i += 1;
                while i < tokens.len() && depth > 0 {
                    match &tokens[i] {
                        Token::Function(_) | Token::OpenParen => {
                            depth += 1;
                            inner.push(tokens[i].clone());
                        }
                        Token::CloseParen => {
                            depth -= 1;
                            if depth > 0 {
                                inner.push(tokens[i].clone());
                            }
                        }
                        other => inner.push(other.clone()),
                    }
                    i += 1;
                }
                if (fn_lower == "rgb" || fn_lower == "rgba")
                    && let Some(c) = parse_rgb_function(&inner)
                {
                    values.push(PropertyValue::Color(c));
                } else if fn_lower == "var" {
                    if let Some(fallback_vals) = parse_var_fallback(&inner) {
                        values.extend(fallback_vals);
                    } else {
                        values.push(PropertyValue::Keyword(format!("var({})", format_tokens(&inner))));
                    }
                } else {
                    values.push(PropertyValue::Keyword(format!(
                        "{fn_lower}({})",
                        format_tokens(&inner)
                    )));
                }
                continue;
            }
            tok => {
                if let Some(val) = token_to_property_value(tok) {
                    values.push(val);
                }
            }
        }
        i += 1;
    }
    values
}

fn parse_var_fallback(inner: &[Token]) -> Option<Vec<PropertyValue>> {
    let mut depth = 0usize;
    let mut comma_idx = None;
    for (idx, tok) in inner.iter().enumerate() {
        match tok {
            Token::Function(_) | Token::OpenParen => depth += 1,
            Token::CloseParen => depth = depth.saturating_sub(1),
            Token::Comma if depth == 0 => {
                comma_idx = Some(idx);
                break;
            }
            _ => {}
        }
    }
    let idx = comma_idx?;
    let rest = &inner[idx + 1..];
    let parsed = parse_value_list(rest);
    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

fn format_tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    for (idx, tok) in tokens.iter().enumerate() {
        if idx > 0
            && !matches!(tok, Token::Comma | Token::CloseParen)
            && !matches!(tokens.get(idx - 1), Some(Token::Function(_)) | Some(Token::OpenParen))
        {
            out.push(' ');
        }
        match tok {
            Token::Ident(s) | Token::AtKeyword(s) => out.push_str(s),
            Token::Hash(s) => {
                out.push('#');
                out.push_str(s);
            }
            Token::String(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            Token::Number(n) => out.push_str(&format!("{n}")),
            Token::Dimension(n, u) => {
                let unit = match u {
                    DimensionUnit::Px => "px",
                    DimensionUnit::Percent => "%",
                    DimensionUnit::Em => "em",
                    DimensionUnit::Rem => "rem",
                    DimensionUnit::Pt => "pt",
                    DimensionUnit::Vw => "vw",
                    DimensionUnit::Vh => "vh",
                };
                out.push_str(&format!("{n}{unit}"));
            }
            Token::Function(f) => {
                out.push_str(f);
                out.push('(');
            }
            Token::OpenParen => out.push('('),
            Token::CloseParen => out.push(')'),
            Token::Comma => out.push(','),
            Token::Star => out.push('*'),
            Token::Slash => out.push('/'),
            Token::Colon => out.push(':'),
            Token::Semicolon => out.push(';'),
            _ => {}
        }
    }
    out
}

fn summarize_grid_template_columns(tokens: &[Token], values: &[PropertyValue]) -> String {
    for val in values {
        if let PropertyValue::Keyword(k) = val {
            let low = k.to_ascii_lowercase();
            if let Some(rest) = low.strip_prefix("repeat(") {
                let first_arg = rest.split(',').next().unwrap_or("").trim();
                if let Ok(n) = first_arg.parse::<f32>()
                    && n >= 1.0
                {
                    return format!("cols:{}", (n.round() as usize).clamp(1, 6));
                }
            }
            if low == "none" || low == "subgrid" {
                return "cols:1".to_string();
            }
        }
    }
    let mut track_count = 0usize;
    let mut depth = 0usize;
    for tok in tokens {
        match tok {
            Token::Function(_) | Token::OpenParen => {
                if depth == 0 {
                    track_count += 1;
                }
                depth += 1;
            }
            Token::CloseParen => depth = depth.saturating_sub(1),
            Token::Dimension(..) | Token::Number(_) | Token::Ident(_) if depth == 0 => {
                track_count += 1;
            }
            _ => {}
        }
    }
    format!("cols:{}", track_count.clamp(1, 6))
}

fn parse_rgb_function(tokens: &[Token]) -> Option<Rgba> {
    let nums: Vec<(f32, bool)> = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Number(n) => Some((*n, false)),
            Token::Dimension(n, DimensionUnit::Percent) => Some((*n, true)),
            _ => None,
        })
        .collect();
    if nums.len() < 3 {
        return None;
    }
    let to_channel = |(v, is_pct): (f32, bool)| -> u8 {
        if is_pct {
            ((v / 100.0) * 255.0).round().clamp(0.0, 255.0) as u8
        } else {
            v.round().clamp(0.0, 255.0) as u8
        }
    };
    let r = to_channel(nums[0]);
    let g = to_channel(nums[1]);
    let b = to_channel(nums[2]);
    let a = if let Some(&(av, is_pct)) = nums.get(3) {
        let frac = if is_pct { av / 100.0 } else { av };
        (frac * 255.0).round().clamp(0.0, 255.0) as u8
    } else {
        255
    };
    Some(Rgba::new(r, g, b, a))
}

fn token_to_property_value(tok: &Token) -> Option<PropertyValue> {
    match tok {
        Token::Dimension(val, DimensionUnit::Px) => Some(PropertyValue::Px(*val)),
        Token::Dimension(val, DimensionUnit::Percent) => Some(PropertyValue::Percent(*val)),
        Token::Dimension(val, DimensionUnit::Em) => Some(PropertyValue::Em(*val)),
        Token::Dimension(val, DimensionUnit::Rem) => Some(PropertyValue::Rem(*val)),
        Token::Dimension(val, DimensionUnit::Pt) => Some(PropertyValue::Pt(*val)),
        Token::Dimension(val, DimensionUnit::Vw) => Some(PropertyValue::Vw(*val)),
        Token::Dimension(val, DimensionUnit::Vh) => Some(PropertyValue::Vh(*val)),
        Token::Number(val) if *val == 0.0 => Some(PropertyValue::Px(0.0)),
        Token::Number(val) => Some(PropertyValue::Number(*val)),
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

fn is_valid_length_or_auto(val: &PropertyValue) -> bool {
    matches!(
        val,
        PropertyValue::Px(_)
            | PropertyValue::Percent(_)
            | PropertyValue::Em(_)
            | PropertyValue::Rem(_)
            | PropertyValue::Pt(_)
            | PropertyValue::Vw(_)
            | PropertyValue::Vh(_)
            | PropertyValue::Auto
    )
}

fn is_length_property(prop: &str) -> bool {
    matches!(
        prop,
        "margin-top"
            | "margin-right"
            | "margin-bottom"
            | "margin-left"
            | "padding-top"
            | "padding-right"
            | "padding-bottom"
            | "padding-left"
            | "width"
            | "height"
            | "min-width"
            | "max-width"
            | "min-height"
            | "border-width"
            | "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width"
            | "border-radius"
            | "font-size"
            | "gap"
    )
}

fn media_query_matches(tokens: &[Token]) -> bool {
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Ident(name) = &tokens[i] {
            let lower = name.to_ascii_lowercase();
            if lower == "print" || lower == "speech" {
                return false;
            }
            if lower == "prefers-color-scheme"
                && tokens.get(i + 1) == Some(&Token::Colon)
                && matches!(tokens.get(i + 2), Some(Token::Ident(v)) if v.eq_ignore_ascii_case("dark"))
            {
                return false;
            }
            if lower == "min-width"
                && tokens.get(i + 1) == Some(&Token::Colon)
                && let Some(px) = media_length_px(tokens.get(i + 2))
                && DEFAULT_MEDIA_VIEWPORT_WIDTH < px
            {
                return false;
            }
            if lower == "max-width"
                && tokens.get(i + 1) == Some(&Token::Colon)
                && let Some(px) = media_length_px(tokens.get(i + 2))
                && DEFAULT_MEDIA_VIEWPORT_WIDTH > px
            {
                return false;
            }
        }
        i += 1;
    }
    true
}

fn media_length_px(tok: Option<&Token>) -> Option<f32> {
    match tok? {
        Token::Dimension(v, DimensionUnit::Px) => Some(*v),
        Token::Dimension(v, DimensionUnit::Em | DimensionUnit::Rem) => Some(*v * 16.0),
        _ => None,
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
        "cyan" | "aqua" => Some(Rgba::new(0, 255, 255, 255)),
        "magenta" | "fuchsia" => Some(Rgba::new(255, 0, 255, 255)),
        "gray" | "grey" => Some(Rgba::new(128, 128, 128, 255)),
        "darkgray" | "darkgrey" => Some(Rgba::new(169, 169, 169, 255)),
        "lightgray" | "lightgrey" => Some(Rgba::new(211, 211, 211, 255)),
        "silver" => Some(Rgba::new(192, 192, 192, 255)),
        "dimgray" | "dimgrey" => Some(Rgba::new(105, 105, 105, 255)),
        "maroon" => Some(Rgba::new(128, 0, 0, 255)),
        "navy" => Some(Rgba::new(0, 0, 128, 255)),
        "olive" => Some(Rgba::new(128, 128, 0, 255)),
        "purple" => Some(Rgba::new(128, 0, 128, 255)),
        "teal" => Some(Rgba::new(0, 128, 128, 255)),
        "orange" => Some(Rgba::new(255, 165, 0, 255)),
        "orangered" => Some(Rgba::new(255, 69, 0, 255)),
        "pink" => Some(Rgba::new(255, 192, 203, 255)),
        "brown" => Some(Rgba::new(165, 42, 42, 255)),
        "gold" => Some(Rgba::new(255, 215, 0, 255)),
        "indigo" => Some(Rgba::new(75, 0, 130, 255)),
        "violet" => Some(Rgba::new(238, 130, 238, 255)),
        "crimson" => Some(Rgba::new(220, 20, 60, 255)),
        "coral" => Some(Rgba::new(255, 127, 80, 255)),
        "tomato" => Some(Rgba::new(255, 99, 71, 255)),
        "whitesmoke" => Some(Rgba::new(245, 245, 245, 255)),
        "aliceblue" => Some(Rgba::new(240, 248, 255, 255)),
        "beige" => Some(Rgba::new(245, 245, 220, 255)),
        "ivory" => Some(Rgba::new(255, 255, 240, 255)),
        "darkblue" => Some(Rgba::new(0, 0, 139, 255)),
        "darkgreen" => Some(Rgba::new(0, 100, 0, 255)),
        "darkred" => Some(Rgba::new(139, 0, 0, 255)),
        "rebeccapurple" => Some(Rgba::new(102, 51, 153, 255)),
        "transparent" => Some(Rgba::new(0, 0, 0, 0)),
        _ => None,
    }
}
