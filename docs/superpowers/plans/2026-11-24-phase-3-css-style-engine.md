# Nirvana Engine Phase 3: CSS Parser & Style Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a CSS3 parser (`nirvana_css`) and style resolution engine (`nirvana_style`) from scratch, transforming raw stylesheets and DOM trees into a fully styled render-ready tree with specificity-sorted cascade and property inheritance.

**Architecture:** A two-crate modular pipeline:
1. `nirvana_css`: Lexer -> Selector/Rule Parser -> `StyleSheet` AST (Selectors with (a,b,c) specificity, Declarations with typed Values).
2. `nirvana_style`: Selector Matcher -> Cascade/Specificity Sorter -> Property Inheritance Resolver -> `StyleTree` (mapping each DOM element to resolved `ComputedValues`).

**Tech Stack:** Pure Rust (Edition 2024 / 1.85+ compatible, tested on Rust 1.99), `nirvana_core`, `nirvana_dom`, `#![forbid(unsafe_code)]`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints
- **Language & Safety:** Pure Rust with `#![forbid(unsafe_code)]` in `nirvana_css` and `nirvana_style`.
- **From Scratch:** All CSS tokenization, selector matching, and cascade sorting written directly from W3C CSS specifications.
- **Memory Model:** Zero cyclic dependencies; `StyleTree` indexes DOM nodes via `NodeId` from `nirvana_dom`.
- **Decoupled Architecture:** `nirvana_css` knows nothing about the DOM; `nirvana_style` depends on both `nirvana_css` and `nirvana_dom`.

## Review Focus
1. **Selector Specificity Ordering:** `#id` (1,0,0) must always beat `.class` (0,1,0), and `.class` must beat `tag` (0,0,1). Same specificity resolves by source order (last rule wins).
2. **User-Agent Default Styles:** Elements like `<div>`, `<p>`, `<h1>` must default to `display: block`, `<span>`/`<a>` to `display: inline`, and `<head>`/`<script>`/`<style>` to `display: none` even when author CSS is empty.
3. **Multi-Class Selector Matching:** `<div class="btn primary">` must match `.btn`, `.primary`, and `.btn.primary`.
4. **Inheritance of Text Properties:** Text properties like `color` must inherit from parent to child unless overridden; box properties (`margin`, `padding`, `border`, `background_color`) must NOT inherit.
5. **Shorthand Margin/Padding Expansion:** `margin: 10px` expands to all 4 sides; `margin: 10px 20px` expands to (top/bottom: 10px, left/right: 20px).

---

### Task 1: `nirvana_css` Crate Setup & CSS Tokenizer

**Files:**
- Modify: `Cargo.toml:4-10`
- Create: `crates/nirvana_css/Cargo.toml`
- Create: `crates/nirvana_css/src/lib.rs`
- Create: `crates/nirvana_css/src/token.rs`
- Create: `crates/nirvana_css/src/tokenizer.rs`
- Test: `crates/nirvana_css/tests/tokenizer_test.rs`

**Interfaces:**
- Consumes: `nirvana_core`
- Produces:
  - `nirvana_css::token::{Token, DimensionUnit}`
  - `nirvana_css::tokenizer::CssTokenizer`
  - `CssTokenizer::new(input: &str) -> CssTokenizer`
  - `CssTokenizer::tokenize_all(&mut self) -> Vec<Token>`

- [ ] **Step 1: Write failing tests in `crates/nirvana_css/tests/tokenizer_test.rs`**

```rust
use nirvana_css::token::{Token, DimensionUnit};
use nirvana_css::tokenizer::CssTokenizer;

#[test]
fn test_tokenize_ruleset_tokens() {
    let css = "h1.title#header { color: #ff0000; margin: 10px; }";
    let tokens = CssTokenizer::new(css).tokenize_all();

    assert!(matches!(&tokens[0], Token::Ident(s) if s == "h1"));
    assert!(matches!(&tokens[1], Token::Dot));
    assert!(matches!(&tokens[2], Token::Ident(s) if s == "title"));
    assert!(matches!(&tokens[3], Token::Hash(s) if s == "header"));
    assert!(matches!(&tokens[4], Token::OpenBrace));
    assert!(matches!(&tokens[5], Token::Ident(s) if s == "color"));
    assert!(matches!(&tokens[6], Token::Colon));
    assert!(matches!(&tokens[7], Token::Hash(s) if s == "ff0000"));
    assert!(matches!(&tokens[8], Token::Semicolon));
    assert!(matches!(&tokens[9], Token::Ident(s) if s == "margin"));
    assert!(matches!(&tokens[10], Token::Colon));
    assert!(matches!(&tokens[11], Token::Dimension(val, DimensionUnit::Px) if *val == 10.0));
    assert!(matches!(&tokens[12], Token::Semicolon));
    assert!(matches!(&tokens[13], Token::CloseBrace));
    assert!(matches!(&tokens[14], Token::Eof));
}

#[test]
fn test_tokenize_comments_and_whitespace_skip() {
    let css = "/* a comment */ div { /* nested */ width: 100px; }";
    let tokens = CssTokenizer::new(css).tokenize_all();

    assert!(matches!(&tokens[0], Token::Ident(s) if s == "div"));
    assert!(matches!(&tokens[1], Token::OpenBrace));
    assert!(matches!(&tokens[2], Token::Ident(s) if s == "width"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_css --test tokenizer_test`
Expected: FAIL (crate not found / member missing)

- [ ] **Step 3: Implement `nirvana_css` token definitions and tokenizer**
  - Add `"crates/nirvana_css"` to workspace `Cargo.toml`.
  - Create `crates/nirvana_css/Cargo.toml` with `nirvana_core`.
  - Implement `Token` enum: `Ident`, `Hash`, `Dot`, `Colon`, `Semicolon`, `Comma`, `OpenBrace`, `CloseBrace`, `Number(f32)`, `Dimension(f32, DimensionUnit)`, `String(String)`, `Eof`.
  - Implement `CssTokenizer`: skips `/* ... */` comments and whitespace, parses identifiers, numbers, units (`px`, `%`, `em`), and symbols.
  - Export in `crates/nirvana_css/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_css --test tokenizer_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_css
git commit -m "feat(css): scaffold nirvana_css crate and css tokenizer"
```

---

### Task 2: `nirvana_css` Parser (Selectors, Declarations & StyleSheet AST)

**Files:**
- Create: `crates/nirvana_css/src/ast.rs`
- Create: `crates/nirvana_css/src/parser.rs`
- Test: `crates/nirvana_css/tests/parser_test.rs`

**Interfaces:**
- Consumes: `nirvana_css::token::*`, `nirvana_core::color::Rgba`
- Produces:
  - `nirvana_css::ast::{StyleSheet, RuleSet, Selector, SimpleSelector, Specificity, Declaration, PropertyValue}`
  - `nirvana_css::parser::parse_css(input: &str) -> StyleSheet`
  - `Selector::specificity(&self) -> Specificity`

- [ ] **Step 1: Write failing tests in `crates/nirvana_css/tests/parser_test.rs`**

```rust
use nirvana_css::parser::parse_css;
use nirvana_css::ast::{SimpleSelector, Specificity, PropertyValue};
use nirvana_core::color::Rgba;

#[test]
fn test_parse_ruleset_and_specificity() {
    let css = r#"
        div.container#main {
            color: #ff0000;
            background-color: #00ff00;
            margin: 10px;
            display: block;
        }
    "#;
    let stylesheet = parse_css(css);
    assert_eq!(stylesheet.rules.len(), 1);

    let rule = &stylesheet.rules[0];
    let selector = &rule.selectors[0];
    assert_eq!(selector.simple_selectors.len(), 3);
    assert!(selector.simple_selectors.contains(&SimpleSelector::Tag("div".into())));
    assert!(selector.simple_selectors.contains(&SimpleSelector::Class("container".into())));
    assert!(selector.simple_selectors.contains(&SimpleSelector::Id("main".into())));

    // Specificity: 1 ID, 1 Class, 1 Tag -> (1, 1, 1)
    assert_eq!(selector.specificity(), Specificity::new(1, 1, 1));

    assert_eq!(rule.get_property("color"), Some(&PropertyValue::Color(Rgba::new(255, 0, 0, 255))));
    assert_eq!(rule.get_property("display"), Some(&PropertyValue::Keyword("block".into())));
}

#[test]
fn test_parse_multiple_selectors_and_shorthand_margins() {
    let css = "h1, h2 { margin: 10px 20px; }";
    let stylesheet = parse_css(css);
    assert_eq!(stylesheet.rules.len(), 1);

    let rule = &stylesheet.rules[0];
    assert_eq!(rule.selectors.len(), 2);
    assert_eq!(rule.selectors[0].specificity(), Specificity::new(0, 0, 1));
    assert_eq!(rule.selectors[1].specificity(), Specificity::new(0, 0, 1));

    // Shorthand margin: 10px 20px -> top/bottom: 10px, left/right: 20px
    assert_eq!(rule.get_property("margin-top"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-bottom"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-left"), Some(&PropertyValue::Px(20.0)));
    assert_eq!(rule.get_property("margin-right"), Some(&PropertyValue::Px(20.0)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_css --test parser_test`
Expected: FAIL

- [ ] **Step 3: Implement `nirvana_css` AST, specificity, and recursive-descent parser**
  - In `crates/nirvana_css/src/ast.rs`:
    - `SimpleSelector`: `Tag(String)`, `Class(String)`, `Id(String)`, `Universal`.
    - `Specificity`: `(ids: u32, classes: u32, tags: u32)`, implements `Ord`, `PartialOrd`, `Eq`, `PartialEq`.
    - `PropertyValue`: `Keyword(String)`, `Color(Rgba)`, `Px(f32)`, `Auto`.
    - `Declaration`: `name: String`, `value: PropertyValue`.
    - `RuleSet`: `selectors: Vec<Selector>`, `declarations: Vec<Declaration>`.
    - `StyleSheet`: `rules: Vec<RuleSet>`.
  - In `crates/nirvana_css/src/parser.rs`:
    - Parse comma-separated selectors.
    - Parse declaration blocks `{ ... }` expanding 1-, 2-, 4-value margin and padding shorthands into `-top`, `-right`, `-bottom`, `-left`.
    - Parse color values via `Rgba::from_hex` or named color lookup (`red`, `blue`, `black`, `white`, `transparent`).
  - Export `parse_css` in `crates/nirvana_css/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_css --test parser_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_css
git commit -m "feat(css): implement css selector, declaration, and stylesheet parser"
```

---

### Task 3: `nirvana_style` Crate & Selector Matcher

**Files:**
- Modify: `Cargo.toml:4-11`
- Create: `crates/nirvana_style/Cargo.toml`
- Create: `crates/nirvana_style/src/lib.rs`
- Create: `crates/nirvana_style/src/matcher.rs`
- Test: `crates/nirvana_style/tests/matcher_test.rs`

**Interfaces:**
- Consumes: `nirvana_css::ast::*`, `nirvana_dom::*`
- Produces:
  - `nirvana_style::matcher::matches_selector(node: NodeId, selector: &Selector, dom: &DomTree) -> bool`

- [ ] **Step 1: Write failing tests in `crates/nirvana_style/tests/matcher_test.rs`**

```rust
use nirvana_dom::DomTree;
use nirvana_css::parser::parse_css;
use nirvana_style::matcher::matches_selector;

#[test]
fn test_selector_matching_tag_class_id() {
    let mut tree = DomTree::new();
    let root = tree.root_id();
    let btn = tree.create_element("button");
    tree.set_attribute(btn, "class", "btn primary").unwrap();
    tree.set_attribute(btn, "id", "submit-btn").unwrap();
    tree.append_child(root, btn).unwrap();

    let css = "button { } .btn { } .primary { } #submit-btn { } .btn.primary { } div { } .secondary { }";
    let stylesheet = parse_css(css);

    // button matches
    assert!(matches_selector(btn, &stylesheet.rules[0].selectors[0], &tree));
    // .btn matches
    assert!(matches_selector(btn, &stylesheet.rules[1].selectors[0], &tree));
    // .primary matches
    assert!(matches_selector(btn, &stylesheet.rules[2].selectors[0], &tree));
    // #submit-btn matches
    assert!(matches_selector(btn, &stylesheet.rules[3].selectors[0], &tree));
    // .btn.primary matches compound
    assert!(matches_selector(btn, &stylesheet.rules[4].selectors[0], &tree));
    // div does not match button
    assert!(!matches_selector(btn, &stylesheet.rules[5].selectors[0], &tree));
    // .secondary does not match
    assert!(!matches_selector(btn, &stylesheet.rules[6].selectors[0], &tree));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_style --test matcher_test`
Expected: FAIL (crate not found / member missing)

- [ ] **Step 3: Implement `nirvana_style` selector matcher**
  - Add `"crates/nirvana_style"` to workspace `Cargo.toml`.
  - Create `crates/nirvana_style/Cargo.toml` with `nirvana_core`, `nirvana_dom`, `nirvana_css`.
  - Implement `matches_selector`:
    - Checks node is `NodeType::Element`.
    - Universal `*` matches any element.
    - Tag matches element `tag_name` case-insensitively.
    - Id matches element `id` attribute.
    - Class matches if `class` attribute (split by whitespace) contains the target class.
    - Compound selector: all simple selectors in the selector must match.
  - Export in `crates/nirvana_style/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_style --test matcher_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_style
git commit -m "feat(style): implement dom selector matching engine"
```

---

### Task 4: Cascade, Specificity Sorting, User-Agent Styles & Computed Style Tree

**Files:**
- Create: `crates/nirvana_style/src/values.rs`
- Create: `crates/nirvana_style/src/user_agent.rs`
- Create: `crates/nirvana_style/src/compute.rs`
- Test: `crates/nirvana_style/tests/style_tree_test.rs`

**Interfaces:**
- Consumes: `nirvana_style::matcher::*`, `nirvana_dom::*`, `nirvana_css::*`
- Produces:
  - `nirvana_style::values::{ComputedValues, Display, Length}`
  - `nirvana_style::compute::{StyleTree, StyledNode, compute_styles}`
  - `compute_styles(dom: &DomTree, stylesheets: &[&StyleSheet]) -> StyleTree`

- [ ] **Step 1: Write failing tests in `crates/nirvana_style/tests/style_tree_test.rs`**

```rust
use nirvana_html::parse_html;
use nirvana_css::parser::parse_css;
use nirvana_style::compute::compute_styles;
use nirvana_style::values::{Display, Length};
use nirvana_core::color::Rgba;

#[test]
fn test_user_agent_default_styles() {
    let html = "<div><p>Paragraph</p><span>Span</span></div>";
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);

    // Find div, p, span nodes
    let div_id = dom.children(dom.children(dom.root_id())[0])[0]; // div under body
    let div_style = style_tree.get_style(div_id).unwrap();
    assert_eq!(div_style.display, Display::Block);

    let children = dom.children(div_id);
    let p_style = style_tree.get_style(children[0]).unwrap();
    assert_eq!(p_style.display, Display::Block);

    let span_style = style_tree.get_style(children[1]).unwrap();
    assert_eq!(span_style.display, Display::Inline);
}

#[test]
fn test_cascade_and_specificity_override() {
    let html = r#"<div id="content" class="box alert">Text</div>"#;
    let dom = parse_html(html);
    let css = r#"
        div { color: #000000; background-color: #ffffff; }
        .box { color: #0000ff; }
        #content { color: #ff0000; }
        .alert { background-color: #ffff00; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let div_id = dom.children(dom.children(dom.root_id())[0])[0];
    let style = style_tree.get_style(div_id).unwrap();

    // ID selector (#content) beats class (.box) and tag (div)
    assert_eq!(style.color, Rgba::new(255, 0, 0, 255));
    // Class selector (.alert) beats tag (div)
    assert_eq!(style.background_color, Rgba::new(255, 255, 0, 255));
}

#[test]
fn test_text_color_inheritance() {
    let html = r#"<div style="color: #ff0000;"><p>Inherited Text</p></div>"#;
    let dom = parse_html(html);
    let css = "div { color: #ff0000; margin-left: 20px; }";
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let div_id = dom.children(dom.children(dom.root_id())[0])[0];
    let p_id = dom.children(div_id)[0];

    let p_style = style_tree.get_style(p_id).unwrap();
    // Color inherits from div
    assert_eq!(p_style.color, Rgba::new(255, 0, 0, 255));
    // Margin does NOT inherit (defaults to 0px)
    assert_eq!(p_style.margin_left, Length::Px(0.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_style --test style_tree_test`
Expected: FAIL

- [ ] **Step 3: Implement User-Agent stylesheet, cascade resolution, and inheritance**
  - Implement `ComputedValues` in `crates/nirvana_style/src/values.rs`:
    - `display: Display` (`Block`, `Inline`, `None`).
    - `color: Rgba` (default black, inherits).
    - `background_color: Rgba` (default transparent, non-inherited).
    - `margin_top/right/bottom/left: Length` (default 0px, non-inherited).
    - `padding_top/right/bottom/left: Length` (default 0px, non-inherited).
    - `border_width: f32` (default 0.0, non-inherited).
    - `width/height: Length` (default Auto, non-inherited).
  - Implement User-Agent defaults in `crates/nirvana_style/src/user_agent.rs` (`html`, `body`, `div`, `p`, `h1`-`h6`, `ul`, `ol`, `li` -> `block`; `span`, `a`, `img` -> `inline`; `head`, `script`, `style` -> `none`).
  - Implement `compute_styles` in `crates/nirvana_style/src/compute.rs`:
    - Collect matching rules from User-Agent stylesheet and Author stylesheets.
    - Sort matched declarations by `(origin: UA=0, Author=1, Specificity, SourceOrder)`.
    - Traverse DOM hierarchy top-down:
      - Inherit inheritable properties (e.g. `color`) from parent's `ComputedValues`.
      - Apply sorted declarations over base values.
      - Store in `StyleTree` indexed by `NodeId`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_style --test style_tree_test`
Expected: PASS

- [ ] **Step 5: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: ALL PASS across all 5 crates (`nirvana_core`, `nirvana_url`, `nirvana_dom`, `nirvana_html`, `nirvana_css`, `nirvana_style`).

- [ ] **Step 6: Commit**

```bash
git add crates/nirvana_style
git commit -m "feat(style): implement style cascade, specificity sorting, and computed style tree"
```
