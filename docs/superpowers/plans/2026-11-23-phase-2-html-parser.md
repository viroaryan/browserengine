# Nirvana Engine Phase 2: HTML5 Tokenizer & Tree Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standards-compliant streaming HTML5 tokenizer state machine and tree construction engine (`nirvana_html`) from scratch, transforming raw HTML text into an arena-backed `nirvana_dom::DomTree`.

**Architecture:** A decoupled streaming pipeline: Character Stream Preprocessor -> Tokenizer State Machine -> Tree Builder (Open Element Stack) -> `DomTree`. Strictly follows the WHATWG HTML specification's tokenization and insertion rules without external parser dependencies.

**Tech Stack:** Rust (Edition 2024 / 1.85+ compatible, tested on Rust 1.99), `nirvana_core`, `nirvana_dom`, `#![forbid(unsafe_code)]`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints
- **Language & Safety:** Pure Rust with `#![forbid(unsafe_code)]` in `nirvana_html`.
- **From Scratch:** All tokenizer state transitions and tree building logic must be hand-crafted from web standards.
- **Memory Model:** Emits nodes into `nirvana_dom::DomTree` via typed `NodeId` without reference cycles.
- **Error Tolerance:** Web HTML is frequently malformed; parser must recover gracefully (auto-closing void tags, implicit container creation) rather than panicking or aborting.

## Review Focus
1. **Void Element Auto-Closing:** Elements like `<img>`, `<br>`, `<meta>`, `<input>`, `<hr>` must never remain on the open element stack or capture subsequent children.
2. **Attribute Case-Insensitivity:** Attribute names in HTML tags must be normalized to lowercase during tokenization (`<DIV CLASS="foo">` -> `class="foo"`).
3. **Unquoted Attribute Value Parsing:** Attribute values without quotes (`<a href=http://example.com>`) must terminate properly at whitespace or `>`.
4. **Nested Tag Tree Hierarchy:** Implicit closing when meeting closing tags higher in the stack (`<div><p>one<div>two</div></div>` must close the `<p>`).
5. **Character Entity Decoding (Basic):** Common entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`) in text and attribute values decoded to their literal characters.

---

### Task 1: `nirvana_html` Crate Setup & Token Definitions

**Files:**
- Modify: `Cargo.toml:4-9`
- Create: `crates/nirvana_html/Cargo.toml`
- Create: `crates/nirvana_html/src/lib.rs`
- Create: `crates/nirvana_html/src/token.rs`
- Test: `crates/nirvana_html/tests/token_test.rs`

**Interfaces:**
- Consumes: `nirvana_core`, `nirvana_dom`
- Produces:
  - `nirvana_html::token::{Token, TagData, DoctypeData}`
  - `Token::StartTag(TagData)`, `Token::EndTag(TagData)`, `Token::Character(char)`, `Token::Comment(String)`, `Token::Doctype(DoctypeData)`, `Token::Eof`

- [ ] **Step 1: Write failing tests in `crates/nirvana_html/tests/token_test.rs`**

```rust
use nirvana_html::token::{Token, TagData, DoctypeData};

#[test]
fn test_token_creation_and_attributes() {
    let mut tag = TagData::new("div", false);
    tag.add_attribute("class", "container");
    tag.add_attribute("ID", "main");

    assert_eq!(tag.name, "div");
    assert_eq!(tag.get_attribute("class"), Some("container"));
    assert_eq!(tag.get_attribute("id"), Some("main")); // Case-insensitive check

    let token = Token::StartTag(tag);
    assert!(matches!(token, Token::StartTag(_)));
}

#[test]
fn test_doctype_token() {
    let doctype = DoctypeData::new("html");
    let token = Token::Doctype(doctype);
    if let Token::Doctype(d) = token {
        assert_eq!(d.name, "html");
    } else {
        panic!("Expected Doctype token");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_html`
Expected: FAIL (crate not found / member missing)

- [ ] **Step 3: Implement `nirvana_html` token structures and register in workspace**
  - Add `"crates/nirvana_html"` to workspace `Cargo.toml`.
  - Create `crates/nirvana_html/Cargo.toml` with `nirvana_core` and `nirvana_dom` dependencies.
  - Implement `TagData`, `DoctypeData`, and `Token` in `crates/nirvana_html/src/token.rs`.
  - Export in `crates/nirvana_html/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_html`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_html
git commit -m "feat(html): scaffold nirvana_html crate and token models"
```

---

### Task 2: HTML5 Streaming Tokenizer State Machine

**Files:**
- Create: `crates/nirvana_html/src/tokenizer.rs`
- Create: `crates/nirvana_html/src/entities.rs`
- Test: `crates/nirvana_html/tests/tokenizer_test.rs`

**Interfaces:**
- Consumes: `nirvana_html::token::*`
- Produces:
  - `nirvana_html::tokenizer::Tokenizer`
  - `Tokenizer::new(input: &str) -> Tokenizer`
  - `Tokenizer::next_token(&mut self) -> Token`
  - `Tokenizer::tokenize_all(&mut self) -> Vec<Token>`

- [ ] **Step 1: Write failing tests in `crates/nirvana_html/tests/tokenizer_test.rs`**

```rust
use nirvana_html::tokenizer::Tokenizer;
use nirvana_html::token::Token;

#[test]
fn test_tokenize_simple_tags_and_text() {
    let mut tokenizer = Tokenizer::new("<p class=\"intro\">Hello &amp; World</p>");
    let tokens = tokenizer.tokenize_all();

    // 1. StartTag(p, class="intro")
    // 2. Character('H'), 'e', 'l', 'l', 'o', ' ', '&', ' ', 'W', 'o', 'r', 'l', 'd'
    // 3. EndTag(p)
    // 4. Eof
    assert!(matches!(&tokens[0], Token::StartTag(tag) if tag.name == "p" && tag.get_attribute("class") == Some("intro")));
    assert!(matches!(&tokens[tokens.len() - 2], Token::EndTag(tag) if tag.name == "p"));
    assert!(matches!(&tokens[tokens.len() - 1], Token::Eof));
}

#[test]
fn test_tokenize_self_closing_and_unquoted_attributes() {
    let mut tokenizer = Tokenizer::new("<img src=logo.png alt='Logo' />");
    let tokens = tokenizer.tokenize_all();

    if let Token::StartTag(tag) = &tokens[0] {
        assert_eq!(tag.name, "img");
        assert_eq!(tag.get_attribute("src"), Some("logo.png"));
        assert_eq!(tag.get_attribute("alt"), Some("Logo"));
        assert!(tag.self_closing);
    } else {
        panic!("Expected StartTag for img");
    }
}

#[test]
fn test_tokenize_comments_and_doctype() {
    let mut tokenizer = Tokenizer::new("<!DOCTYPE html><!-- a comment --><span>text</span>");
    let tokens = tokenizer.tokenize_all();

    assert!(matches!(&tokens[0], Token::Doctype(d) if d.name == "html"));
    assert!(matches!(&tokens[1], Token::Comment(c) if c == " a comment "));
    assert!(matches!(&tokens[2], Token::StartTag(tag) if tag.name == "span"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_html --test tokenizer_test`
Expected: FAIL

- [ ] **Step 3: Implement Tokenizer state machine and entity decoding**
  - Implement `crates/nirvana_html/src/entities.rs`: replace `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` with decoded chars.
  - Implement `crates/nirvana_html/src/tokenizer.rs`:
    - States: `Data`, `TagOpen`, `EndTagOpen`, `TagName`, `BeforeAttributeName`, `AttributeName`, `AfterAttributeName`, `BeforeAttributeValue`, `AttributeValueDoubleQuoted`, `AttributeValueSingleQuoted`, `AttributeValueUnquoted`, `AfterAttributeValueQuoted`, `SelfClosingStartTag`, `MarkupDeclarationOpen`, `Comment`, `Doctype`.
    - Handle EOF transitions and buffer accumulation.
    - Export `Tokenizer` in `crates/nirvana_html/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_html --test tokenizer_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_html
git commit -m "feat(html): implement HTML5 streaming tokenizer state machine"
```

---

### Task 3: HTML Tree Builder (Tree Construction Dispatcher)

**Files:**
- Create: `crates/nirvana_html/src/tree_builder.rs`
- Test: `crates/nirvana_html/tests/tree_builder_test.rs`

**Interfaces:**
- Consumes: `nirvana_html::tokenizer::Tokenizer`, `nirvana_dom::DomTree`
- Produces:
  - `nirvana_html::tree_builder::TreeBuilder`
  - `TreeBuilder::new(tokens: Vec<Token>) -> TreeBuilder`
  - `TreeBuilder::build(mut self) -> DomTree`

- [ ] **Step 1: Write failing tests in `crates/nirvana_html/tests/tree_builder_test.rs`**

```rust
use nirvana_html::tokenizer::Tokenizer;
use nirvana_html::tree_builder::TreeBuilder;
use nirvana_dom::node::NodeType;

#[test]
fn test_tree_construction_nested_document() {
    let html = "<html><head><title>Nirvana</title></head><body><h1>Title</h1><p>Paragraph</p></body></html>";
    let tokens = Tokenizer::new(html).tokenize_all();
    let tree = TreeBuilder::new(tokens).build();

    let root = tree.root_id();
    let children = tree.children(root);
    assert_eq!(children.len(), 1); // <html>
    let html_id = children[0];
    assert_eq!(tree.get_node(html_id).unwrap().node_type(), NodeType::Element);

    let html_children = tree.children(html_id); // <head>, <body>
    assert_eq!(html_children.len(), 2);
}

#[test]
fn test_tree_void_elements_do_not_capture_children() {
    let html = "<div><img src=\"test.png\"><p>After Image</p></div>";
    let tokens = Tokenizer::new(html).tokenize_all();
    let tree = TreeBuilder::new(tokens).build();

    let root = tree.root_id();
    let html_node = tree.children(root)[0];
    let body_node = tree.children(html_node).iter().find(|&&id| {
        if let nirvana_dom::node::NodeData::Element(data) = &tree.get_node(id).unwrap().data {
            data.tag_name == "body"
        } else {
            false
        }
    }).copied().unwrap_or(html_node);

    let div_id = tree.children(body_node)[0];
    let div_children = tree.children(div_id);
    assert_eq!(div_children.len(), 2); // <img> and <p> are siblings under <div>
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_html --test tree_builder_test`
Expected: FAIL

- [ ] **Step 3: Implement TreeBuilder with open element stack and void tag handling**
  - Maintain `open_elements: Vec<NodeId>`.
  - Maintain void elements list (`area`, `base`, `br`, `col`, `embed`, `hr`, `img`, `input`, `link`, `meta`, `source`, `track`, `wbr`).
  - Auto-generate implicit `<html>` and `<body>` if omitted in fragments.
  - On StartTag:
    - Create element in `DomTree`.
    - Append as child of `current_node()` (top of open elements stack).
    - If not a void element and not self-closing, push onto `open_elements`.
  - On EndTag:
    - Pop from `open_elements` matching the tag name (or nearest matching ancestor).
  - On Character:
    - Append text to current open element (coalescing contiguous text tokens).
  - On Comment:
    - Create comment in `DomTree` and append to `current_node()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_html --test tree_builder_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_html
git commit -m "feat(html): implement tree builder with open element stack"
```

---

### Task 4: High-Level `parse_html` API & Error Recovery

**Files:**
- Modify: `crates/nirvana_html/src/lib.rs`
- Test: `crates/nirvana_html/tests/parser_integration_test.rs`

**Interfaces:**
- Consumes: `nirvana_html::tokenizer::Tokenizer`, `nirvana_html::tree_builder::TreeBuilder`
- Produces:
  - `nirvana_html::parse_html(input: &str) -> DomTree`

- [ ] **Step 1: Write failing tests in `crates/nirvana_html/tests/parser_integration_test.rs`**

```rust
use nirvana_html::parse_html;
use nirvana_dom::node::NodeData;

#[test]
fn test_parse_html_convenience_api() {
    let tree = parse_html("<div class=\"hero\"><h1>Welcome to Nirvana Browser</h1></div>");
    let root = tree.root_id();
    assert!(!tree.children(root).is_empty());
}

#[test]
fn test_parse_html_malformed_unclosed_tags() {
    // Unclosed <p> tags should be handled without panic or orphan nodes
    let tree = parse_html("<div><p>First paragraph<div>Nested block</div>");
    let root = tree.root_id();
    assert!(!tree.children(root).is_empty());
}

#[test]
fn test_parse_html_attribute_retrieval() {
    let tree = parse_html("<a href=\"https://nirvanabrowser.org\" target=\"_blank\">Link</a>");
    let root = tree.root_id();
    
    // Find the <a> tag and verify its attributes
    let mut found = false;
    let mut stack = vec![root];
    while let Some(node_id) = stack.pop() {
        if let Some(node) = tree.get_node(node_id) {
            if let NodeData::Element(data) = &node.data {
                if data.tag_name == "a" {
                    assert_eq!(tree.get_attribute(node_id, "href"), Some("https://nirvanabrowser.org"));
                    assert_eq!(tree.get_attribute(node_id, "target"), Some("_blank"));
                    found = true;
                    break;
                }
            }
            stack.extend(tree.children(node_id));
        }
    }
    assert!(found);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_html --test parser_integration_test`
Expected: FAIL (`parse_html` not defined)

- [ ] **Step 3: Implement `parse_html` in `crates/nirvana_html/src/lib.rs`**
  - Expose `pub fn parse_html(html: &str) -> DomTree`:
    ```rust
    pub fn parse_html(html: &str) -> DomTree {
        let tokens = Tokenizer::new(html).tokenize_all();
        TreeBuilder::new(tokens).build()
    }
    ```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_html --test parser_integration_test`
Expected: PASS

- [ ] **Step 5: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: ALL PASS across all 4 crates (`nirvana_core`, `nirvana_url`, `nirvana_dom`, `nirvana_html`).

- [ ] **Step 6: Commit**

```bash
git add crates/nirvana_html
git commit -m "feat(html): expose high-level parse_html API with error recovery"
```
