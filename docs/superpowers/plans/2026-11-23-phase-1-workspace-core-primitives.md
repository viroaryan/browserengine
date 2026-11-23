# Nirvana Engine Phase 1: Workspace & Core Primitives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational Cargo workspace, core geometry/color/arena primitives, from-scratch WHATWG URL parser, and arena-backed DOM tree data structures for the Nirvana Browser Engine.

**Architecture:** A pure Rust monorepo workspace containing decoupled crates with zero circular dependencies. Memory-safe, cache-friendly index-based arenas (`Id<T>`) represent hierarchical tree structures without reference cycles, guaranteeing thread-safe (`Send + Sync`) traversal.

**Tech Stack:** Rust (Edition 2024 / 1.85+ compatible, tested on Rust 1.99), Cargo Workspaces, `#![forbid(unsafe_code)]`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints
- **Language & Toolchain:** Pure Rust (Cargo workspace) targeting host and Android cross-compilation.
- **Unsafe Code:** `#![forbid(unsafe_code)]` in all crates implemented in this phase.
- **No Heavy Third-Party Parser Wrappers:** URL and DOM structures must be implemented from foundational principles.
- **Memory Model:** Zero reference counting cycles; all node relationships managed via typed IDs (`NodeId`).

## Review Focus
1. **URL Path Normalization:** Relative dot-segments (`/a/b/../c` -> `/a/c`) and backslashes in path segment parsing.
2. **URL Default Port Resolution:** Port omission for standard schemes (`http://host` -> port 80, `https://host` -> port 443).
3. **DOM Tree Cycle Prevention:** Attempting to append an ancestor as a child of a descendant must return an error.
4. **Arena ID Invalidation:** Removing or querying invalid IDs must not panic or return mismatched node data.
5. **Color Hex Parsing Malformed Inputs:** `#xyz`, `#12`, or `#12345` must fail cleanly with descriptive errors rather than panic.

---

### Task 1: Cargo Workspace Initialization & Root Config

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`

**Interfaces:**
- Consumes: None
- Produces: Root workspace manifest defining crate members and shared compiler lints.

- [ ] **Step 1: Write root workspace configuration in `Cargo.toml` and `.gitignore`**

Create `Cargo.toml` with:
```toml
[workspace]
members = [
    "crates/nirvana_core",
    "crates/nirvana_url",
    "crates/nirvana_dom",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Aryan <aryan@nirvanabrowser.local>"]
license = "MIT OR Apache-2.0"

[workspace.lints.rust]
unsafe_code = "forbid"
```

Create `.gitignore` for target directory, `.idea`, and editor temporary files.

- [ ] **Step 2: Verify workspace syntax**

Run: `cargo check` (Expected: warning/fail about missing member paths until crates exist).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml .gitignore
git commit -m "chore: initialize root cargo workspace"
```

---

### Task 2: `nirvana_core` Primitives (Geometry, Color & Index Arena)

**Files:**
- Create: `crates/nirvana_core/Cargo.toml`
- Create: `crates/nirvana_core/src/lib.rs`
- Create: `crates/nirvana_core/src/geometry.rs`
- Create: `crates/nirvana_core/src/color.rs`
- Create: `crates/nirvana_core/src/arena.rs`
- Test: `crates/nirvana_core/tests/primitives_test.rs`

**Interfaces:**
- Consumes: None
- Produces:
  - `nirvana_core::geometry::{Point, Size, Rect}`
  - `nirvana_core::color::Rgba` with `Rgba::from_hex(&str) -> Result<Rgba, ColorError>`
  - `nirvana_core::arena::{Arena<T>, Id<T>}` with `alloc()`, `get()`, `get_mut()`, `iter()`

- [ ] **Step 1: Write failing tests for Geometry, Color, and Arena in `crates/nirvana_core/tests/primitives_test.rs`**

```rust
use nirvana_core::geometry::{Point, Size, Rect};
use nirvana_core::color::Rgba;
use nirvana_core::arena::Arena;

#[test]
fn test_geometry_containment_and_intersection() {
    let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert!(r1.contains_point(Point::new(50.0, 50.0)));
    assert!(!r1.contains_point(Point::new(150.0, 50.0)));
}

#[test]
fn test_color_hex_parsing() {
    assert_eq!(Rgba::from_hex("#ff0000").unwrap(), Rgba::new(255, 0, 0, 255));
    assert_eq!(Rgba::from_hex("#00ff007f").unwrap(), Rgba::new(0, 255, 0, 127));
    assert!(Rgba::from_hex("#xyz").is_err());
    assert!(Rgba::from_hex("#12").is_err());
}

#[test]
fn test_arena_storage() {
    let mut arena: Arena<String> = Arena::new();
    let id1 = arena.alloc("Node1".to_string());
    let id2 = arena.alloc("Node2".to_string());
    assert_eq!(arena.get(id1), Some(&"Node1".to_string()));
    assert_eq!(arena.get(id2), Some(&"Node2".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_core`
Expected: FAIL (crate / modules not found)

- [ ] **Step 3: Implement `nirvana_core` modules**
  - In `crates/nirvana_core/src/geometry.rs`: implement `Point<f32>`, `Size<f32>`, `Rect<f32>` with `contains_point`, `intersects`, `union`.
  - In `crates/nirvana_core/src/color.rs`: implement `Rgba` (u8 channels) with hex parsing `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`.
  - In `crates/nirvana_core/src/arena.rs`: implement `Arena<T>` with `Vec<Option<T>>` and strongly-typed `Id<T>(u32, PhantomData<T>)`.
  - In `crates/nirvana_core/src/lib.rs`: export all modules with `#![forbid(unsafe_code)]`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_core`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_core
git commit -m "feat(core): implement geometry, color, and typed arena primitives"
```

---

### Task 3: `nirvana_url` WHATWG URL Parser

**Files:**
- Create: `crates/nirvana_url/Cargo.toml`
- Create: `crates/nirvana_url/src/lib.rs`
- Create: `crates/nirvana_url/src/parser.rs`
- Create: `crates/nirvana_url/src/error.rs`
- Test: `crates/nirvana_url/tests/url_test.rs`

**Interfaces:**
- Consumes: `nirvana_core`
- Produces:
  - `nirvana_url::Url`
  - `Url::parse(input: &str) -> Result<Url, UrlError>`
  - `url.scheme() -> &str`, `url.host() -> Option<&str>`, `url.port() -> u16`, `url.path() -> &str`, `url.query() -> Option<&str>`, `url.fragment() -> Option<&str>`

- [ ] **Step 1: Write failing tests for URL parsing in `crates/nirvana_url/tests/url_test.rs`**

```rust
use nirvana_url::{Url, UrlError};

#[test]
fn test_url_standard_http() {
    let url = Url::parse("http://example.com/index.html?search=nirvana#section1").unwrap();
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host(), Some("example.com"));
    assert_eq!(url.port(), 80);
    assert_eq!(url.path(), "/index.html");
    assert_eq!(url.query(), Some("search=nirvana"));
    assert_eq!(url.fragment(), Some("section1"));
}

#[test]
fn test_url_custom_port() {
    let url = Url::parse("https://example.com:8443/api").unwrap();
    assert_eq!(url.port(), 8443);
}

#[test]
fn test_url_file_scheme() {
    let url = Url::parse("file:///sdcard/Download/test.html").unwrap();
    assert_eq!(url.scheme(), "file");
    assert_eq!(url.path(), "/sdcard/Download/test.html");
}

#[test]
fn test_url_path_dot_normalization() {
    let url = Url::parse("http://example.com/a/b/../c").unwrap();
    assert_eq!(url.path(), "/a/c");
}

#[test]
fn test_url_invalid_input() {
    assert!(matches!(Url::parse("not-a-url"), Err(UrlError::MissingScheme)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_url`
Expected: FAIL (crate or functions not defined)

- [ ] **Step 3: Implement `nirvana_url` parser state machine**
  - Implement state machine in `crates/nirvana_url/src/parser.rs`:
    - Scheme state: parse letters, digits, `+`, `-`, `.` followed by `:`.
    - Host & Authority state: parse `//` authority, extract host and optional `:port`.
    - Path state: parse `/`-separated segments with relative dot-segment normalization (`.` and `..`).
    - Query state: extract after `?`.
    - Fragment state: extract after `#`.
    - Default port resolver: `http` -> 80, `https` -> 443, `file` -> 0.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_url`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_url
git commit -m "feat(url): implement WHATWG url parser from scratch"
```

---

### Task 4: `nirvana_dom` Arena-Backed DOM Hierarchy

**Files:**
- Create: `crates/nirvana_dom/Cargo.toml`
- Create: `crates/nirvana_dom/src/lib.rs`
- Create: `crates/nirvana_dom/src/node.rs`
- Create: `crates/nirvana_dom/src/tree.rs`
- Test: `crates/nirvana_dom/tests/dom_tree_test.rs`

**Interfaces:**
- Consumes: `nirvana_core::arena::Arena`
- Produces:
  - `nirvana_dom::node::{Node, NodeData, ElementData, NodeType}`
  - `nirvana_dom::tree::{DomTree, NodeId, DomError}`
  - `DomTree::new() -> DomTree`
  - `DomTree::create_element(&mut self, tag: &str) -> NodeId`
  - `DomTree::create_text(&mut self, text: &str) -> NodeId`
  - `DomTree::create_comment(&mut self, comment: &str) -> NodeId`
  - `DomTree::append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DomError>`
  - `DomTree::children(&self, parent: NodeId) -> Vec<NodeId>`
  - `DomTree::parent(&self, node: NodeId) -> Option<NodeId>`

- [ ] **Step 1: Write failing tests in `crates/nirvana_dom/tests/dom_tree_test.rs`**

```rust
use nirvana_dom::tree::{DomTree, DomError};
use nirvana_dom::node::NodeType;

#[test]
fn test_dom_tree_construction_and_traversal() {
    let mut tree = DomTree::new();
    let root = tree.root_id();
    
    let html = tree.create_element("html");
    let body = tree.create_element("body");
    let p = tree.create_element("p");
    let text = tree.create_text("Hello Nirvana Engine");

    tree.append_child(root, html).unwrap();
    tree.append_child(html, body).unwrap();
    tree.append_child(body, p).unwrap();
    tree.append_child(p, text).unwrap();

    assert_eq!(tree.parent(text), Some(p));
    assert_eq!(tree.parent(p), Some(body));
    assert_eq!(tree.children(body), vec![p]);
    assert_eq!(tree.children(p), vec![text]);
}

#[test]
fn test_dom_cycle_prevention() {
    let mut tree = DomTree::new();
    let div1 = tree.create_element("div");
    let div2 = tree.create_element("div");

    tree.append_child(div1, div2).unwrap();
    // Attempting to append parent as a child of its own child must fail
    assert!(matches!(tree.append_child(div2, div1), Err(DomError::CycleDetected)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_dom`
Expected: FAIL

- [ ] **Step 3: Implement `nirvana_dom` node structures and tree hierarchy**
  - Implement `Node` in `crates/nirvana_dom/src/node.rs` containing `parent: Option<NodeId>`, `first_child: Option<NodeId>`, `last_child: Option<NodeId>`, `prev_sibling: Option<NodeId>`, `next_sibling: Option<NodeId>`, and `data: NodeData`.
  - Implement `DomTree` in `crates/nirvana_dom/src/tree.rs` backed by `nirvana_core::arena::Arena<Node>`.
  - Implement cycle validation in `append_child`: traverse parents of `parent` to verify `child` is not an ancestor.
  - Implement clean sibling pointer updates when linking/unlinking nodes.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_dom`
Expected: PASS

- [ ] **Step 5: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/nirvana_dom
git commit -m "feat(dom): implement arena-backed DOM tree with cycle prevention"
```
