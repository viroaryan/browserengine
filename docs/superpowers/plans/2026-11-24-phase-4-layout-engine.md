# Nirvana Engine Phase 4: Layout Engine & Box Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a CSS2.1 / CSS3 box model and block layout engine (`nirvana_layout`) from scratch, transforming a styled DOM tree (`DomTree` + `StyleTree`) into an exact pixel-positioned `LayoutBox` tree ready for 2D painting.

**Architecture:** A modular layout calculation pipeline:
1. `dimensions`: Strongly-typed CSS Box Model representations (`EdgeSizes`, `Dimensions`, `Rect`) with `padding_box()`, `border_box()`, and `margin_box()` calculations.
2. `tree_builder`: Layout tree construction mapping styled DOM elements to `LayoutBox` nodes (`BlockNode`, `InlineNode`, `AnonymousBlock`), filtering out `Display::None` elements and coalescing inline nodes when required by CSS 2.1 §9.2.1.1.
3. `block`: Normal flow Block Formatting Context (BFC) layout implementing W3C CSS 2.1 §10.3.3 (width calculation, `margin: auto` horizontal centering, underflow resolution, and vertical height accumulation).
4. `layout`: Viewport-based document layout entry point transforming `(DomTree, StyleTree, Viewport)` into the root containing block layout.

**Tech Stack:** Pure Rust (Edition 2024 / 1.85+ compatible, tested on Rust 1.99), `nirvana_core`, `nirvana_dom`, `nirvana_css`, `nirvana_style`, `#![forbid(unsafe_code)]`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints
- **Language & Safety:** Pure Rust with `#![forbid(unsafe_code)]` in `nirvana_layout`.
- **From Scratch:** All box model arithmetic, width resolution, margin auto-centering, and BFC stacking written directly from W3C CSS 2.1 specifications.
- **Decoupled Architecture:** `nirvana_layout` consumes `nirvana_core`, `nirvana_dom`, `nirvana_style`, and `nirvana_css`. It does not know about painting or GPU graphics (pure geometry).
- **Strict Coordinate System:** Top-left origin `(0.0, 0.0)` with positive X rightward and positive Y downward.

## Review Focus
1. **`Display::None` Pruning:** `<head>`, `<script>`, `<style>`, and any element with `display: none` must generate ZERO layout boxes and consume zero space.
2. **`margin: auto` Centering:** A block with explicit `width` (e.g. `width: 400px; margin: 0 auto;`) inside a `800px` container must split excess width equally between `margin-left` and `margin-right` ($200\text{px}$ each).
3. **Width Expansion (`width: auto`):** In normal flow, a block with `width: auto` must expand its content width such that `margin_box().width == containing_block.content.width`.
4. **Vertical Stacking in Normal Flow:** Child blocks must stack strictly vertically, where $\text{child}[i].y = \text{child}[i-1].y + \text{child}[i-1].\text{margin\_box}().\text{height}$.
5. **Auto Height Accumulation:** A parent block with `height: auto` must have its content height computed as the sum of all its children's margin boxes.

---

### Task 1: `nirvana_layout` Crate Setup & Box Model Geometry

**Files:**
- Modify: `Cargo.toml:4-12`
- Create: `crates/nirvana_layout/Cargo.toml`
- Create: `crates/nirvana_layout/src/lib.rs`
- Create: `crates/nirvana_layout/src/dimensions.rs`
- Test: `crates/nirvana_layout/tests/dimensions_test.rs`

**Interfaces:**
- Consumes: `nirvana_core::geometry::{Point, Rect, Size}`
- Produces:
  - `nirvana_layout::dimensions::{Dimensions, EdgeSizes}`
  - `Dimensions::padding_box(&self) -> Rect`
  - `Dimensions::border_box(&self) -> Rect`
  - `Dimensions::margin_box(&self) -> Rect`

- [ ] **Step 1: Write failing tests in `crates/nirvana_layout/tests/dimensions_test.rs`**

```rust
use nirvana_core::geometry::Rect;
use nirvana_layout::dimensions::{Dimensions, EdgeSizes};

#[test]
fn test_box_model_areas() {
    let mut d = Dimensions::default();
    d.content = Rect::new(50.0, 50.0, 200.0, 100.0);
    d.padding = EdgeSizes { top: 10.0, right: 10.0, bottom: 10.0, left: 10.0 };
    d.border = EdgeSizes { top: 2.0, right: 2.0, bottom: 2.0, left: 2.0 };
    d.margin = EdgeSizes { top: 20.0, right: 15.0, bottom: 20.0, left: 15.0 };

    // Padding box: content expanded by padding
    let p_box = d.padding_box();
    assert_eq!(p_box, Rect::new(40.0, 40.0, 220.0, 120.0));

    // Border box: padding box expanded by border
    let b_box = d.border_box();
    assert_eq!(b_box, Rect::new(38.0, 38.0, 224.0, 124.0));

    // Margin box: border box expanded by margin
    let m_box = d.margin_box();
    assert_eq!(m_box, Rect::new(23.0, 18.0, 254.0, 164.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_layout --test dimensions_test`
Expected: FAIL (crate not found)

- [ ] **Step 3: Implement `nirvana_layout` dimensions**
  - Add `"crates/nirvana_layout"` to root `Cargo.toml`.
  - Create `crates/nirvana_layout/Cargo.toml` with `nirvana_core`, `nirvana_dom`, `nirvana_style`.
  - Implement `EdgeSizes` (`top`, `right`, `bottom`, `left`).
  - Implement `Dimensions` with `padding_box()`, `border_box()`, and `margin_box()`.
  - Export in `crates/nirvana_layout/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_layout --test dimensions_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_layout
git commit -m "feat(layout): implement css box model dimensions and geometry"
```

---

### Task 2: Layout Tree Construction (`tree_builder.rs`)

**Files:**
- Create: `crates/nirvana_layout/src/box_type.rs`
- Create: `crates/nirvana_layout/src/tree_builder.rs`
- Test: `crates/nirvana_layout/tests/tree_builder_test.rs`

**Interfaces:**
- Consumes: `nirvana_dom::*`, `nirvana_style::*`, `nirvana_layout::dimensions::*`
- Produces:
  - `nirvana_layout::box_type::{BoxType, LayoutBox}`
  - `nirvana_layout::tree_builder::build_layout_tree(dom: &DomTree, style_tree: &StyleTree) -> Option<LayoutBox>`

- [ ] **Step 1: Write failing tests in `crates/nirvana_layout/tests/tree_builder_test.rs`**

```rust
use nirvana_html::parse_html;
use nirvana_style::compute::compute_styles;
use nirvana_layout::box_type::BoxType;
use nirvana_layout::tree_builder::build_layout_tree;

#[test]
fn test_build_layout_tree_filtering_and_hierarchy() {
    let html = r#"
        <html>
            <head><title>Ignore me</title></head>
            <body>
                <div id="container">
                    <p>Paragraph</p>
                    <span style="display: none;">Hidden</span>
                </div>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = build_layout_tree(&dom, &style_tree).expect("root layout box");

    // Root should be html (BlockNode)
    assert!(matches!(layout_root.box_type, BoxType::BlockNode(_)));

    // Head is display: none so it must NOT be in the layout tree!
    // html should only have body as a child in layout
    assert_eq!(layout_root.children.len(), 1);
    let body_box = &layout_root.children[0];
    assert!(matches!(body_box.box_type, BoxType::BlockNode(_)));

    // Body should have container
    assert_eq!(body_box.children.len(), 1);
    let container_box = &body_box.children[0];

    // Container should contain p, but NOT the hidden span!
    assert_eq!(container_box.children.len(), 1);
    let p_box = &container_box.children[0];
    assert!(matches!(p_box.box_type, BoxType::BlockNode(_)));
}

#[test]
fn test_anonymous_block_box_insertion() {
    let html = r#"
        <div>
            Some text
            <p>Block child</p>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = build_layout_tree(&dom, &style_tree).expect("root layout box");

    // Find div
    let body_box = &layout_root.children[0];
    let div_box = &body_box.children[0];

    // div contains an inline child (text) and a block child (p)
    // CSS 2.1 §9.2.1.1: inline child must be wrapped in an AnonymousBlock
    assert!(div_box.children.iter().any(|c| matches!(c.box_type, BoxType::AnonymousBlock)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_layout --test tree_builder_test`
Expected: FAIL

- [ ] **Step 3: Implement `BoxType`, `LayoutBox`, and `build_layout_tree`**
  - Implement `BoxType`: `BlockNode(Id<Node>)`, `InlineNode(Id<Node>)`, `AnonymousBlock`.
  - Implement `LayoutBox`: `box_type: BoxType`, `dimensions: Dimensions`, `children: Vec<LayoutBox>`.
  - Implement recursive tree builder:
    - If node has `display: None` (or is inside `<head>`), return `None`.
    - If `display: Block`, construct `BlockNode`.
    - If `display: Inline`, construct `InlineNode`.
    - If a block container has both block and inline children, wrap consecutive inline children in `AnonymousBlock`.
  - Export in `crates/nirvana_layout/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_layout --test tree_builder_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_layout
git commit -m "feat(layout): implement layout tree construction and anonymous block generation"
```

---

### Task 3: Block Formatting Context & Box Sizing (`block.rs`)

**Files:**
- Create: `crates/nirvana_layout/src/block.rs`
- Test: `crates/nirvana_layout/tests/block_layout_test.rs`

**Interfaces:**
- Consumes: `nirvana_style::*`, `nirvana_layout::dimensions::*`, `nirvana_layout::box_type::*`
- Produces:
  - `LayoutBox::layout(&mut self, containing_block: &Dimensions, style_tree: &StyleTree)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_layout/tests/block_layout_test.rs`**

```rust
use nirvana_core::geometry::Rect;
use nirvana_html::parse_html;
use nirvana_css::parser::parse_css;
use nirvana_style::compute::compute_styles;
use nirvana_layout::dimensions::Dimensions;
use nirvana_layout::tree_builder::build_layout_tree;

#[test]
fn test_block_width_auto_expansion() {
    let html = "<div></div>";
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let mut viewport_containing_block = Dimensions::default();
    viewport_containing_block.content = Rect::new(0.0, 0.0, 800.0, 600.0);

    layout_root.layout(&viewport_containing_block, &style_tree);

    // html content width expands to fill containing block (800.0)
    assert_eq!(layout_root.dimensions.content.width, 800.0);
}

#[test]
fn test_block_margin_auto_centering() {
    let html = r#"<div style="width: 400px; margin: 0 auto;"></div>"#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let mut viewport_containing_block = Dimensions::default();
    viewport_containing_block.content = Rect::new(0.0, 0.0, 800.0, 600.0);

    layout_root.layout(&viewport_containing_block, &style_tree);

    // div is under body, under html
    let body_box = &layout_root.children[0];
    let div_box = &body_box.children[0];

    assert_eq!(div_box.dimensions.content.width, 400.0);
    // 800 - 400 = 400 remaining space -> 200 left, 200 right
    assert_eq!(div_box.dimensions.margin.left, 200.0);
    assert_eq!(div_box.dimensions.margin.right, 200.0);
    assert_eq!(div_box.dimensions.content.x, 200.0);
}

#[test]
fn test_block_vertical_stacking_and_auto_height() {
    let html = r#"
        <div id="parent">
            <div style="height: 50px; margin-bottom: 10px;"></div>
            <div style="height: 100px;"></div>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let mut viewport_containing_block = Dimensions::default();
    viewport_containing_block.content = Rect::new(0.0, 0.0, 500.0, 1000.0);

    layout_root.layout(&viewport_containing_block, &style_tree);

    let body_box = &layout_root.children[0];
    let parent_box = &body_box.children[0];
    let child1 = &parent_box.children[0];
    let child2 = &parent_box.children[1];

    assert_eq!(child1.dimensions.content.height, 50.0);
    assert_eq!(child2.dimensions.content.height, 100.0);

    // child 2 stacks below child 1's margin box: 0 + 50 + 10 = 60
    assert_eq!(child2.dimensions.content.y, child1.dimensions.content.y + 50.0 + 10.0);

    // Parent height expands to enclose all children: 50 + 10 + 100 = 160.0
    assert_eq!(parent_box.dimensions.content.height, 160.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_layout --test block_layout_test`
Expected: FAIL

- [ ] **Step 3: Implement normal flow block layout algorithm**
  - Implement `layout_block(&mut self, containing_block: &Dimensions, style_tree: &StyleTree)`:
    - Step 1: Calculate width, padding, border, margins per CSS 2.1 §10.3.3.
    - Step 2: Calculate position (x from margin-left + padding-left + border-left, y from containing block's accumulated height).
    - Step 3: Layout children recursively, accumulating child margin box heights.
    - Step 4: Calculate height (explicit `Px` or auto-sum from children).
  - Export in `crates/nirvana_layout/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_layout --test block_layout_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_layout
git commit -m "feat(layout): implement normal flow block layout and box sizing"
```

---

### Task 4: Complete Document Layout Pipeline & Full Integration

**Files:**
- Create: `crates/nirvana_layout/src/layout.rs`
- Modify: `crates/nirvana_layout/src/lib.rs`
- Test: `crates/nirvana_layout/tests/integration_test.rs`

**Interfaces:**
- Produces:
  - `nirvana_layout::layout::layout_document(dom: &DomTree, style_tree: &StyleTree, viewport: Size) -> Option<LayoutBox>`

- [ ] **Step 1: Write integration tests in `crates/nirvana_layout/tests/integration_test.rs`**

```rust
use nirvana_core::geometry::Size;
use nirvana_html::parse_html;
use nirvana_css::parser::parse_css;
use nirvana_style::compute::compute_styles;
use nirvana_layout::layout::layout_document;

#[test]
fn test_full_document_layout_pipeline() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { margin: 0; padding: 0; }
                    header { height: 60px; background-color: #333333; }
                    main { width: 600px; margin: 20px auto; padding: 10px; }
                    footer { height: 40px; }
                </style>
            </head>
            <body>
                <header></header>
                <main>
                    <p style="height: 30px;"></p>
                </main>
                <footer></footer>
            </body>
        </html>
    "#;

    let dom = parse_html(html);
    // Parse author stylesheet from <style> or directly
    let css = r#"
        body { margin: 0; padding: 0; }
        header { height: 60px; }
        main { width: 600px; margin: 20px auto; padding: 10px; }
        footer { height: 40px; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let viewport = Size::new(1000.0, 800.0);
    let root_box = layout_document(&dom, &style_tree, viewport).expect("root layout box");

    // Root (html) should span viewport width (1000px)
    assert_eq!(root_box.dimensions.content.width, 1000.0);

    let body = &root_box.children[0];
    assert_eq!(body.children.len(), 3); // header, main, footer

    let header = &body.children[0];
    assert_eq!(header.dimensions.content.height, 60.0);
    assert_eq!(header.dimensions.content.y, 0.0);

    let main = &body.children[1];
    assert_eq!(main.dimensions.content.width, 600.0);
    // Auto centered in 1000px viewport: (1000 - (600 + 20 padding)) / 2 = 190.0
    assert_eq!(main.dimensions.margin.left, 190.0);
    assert_eq!(main.dimensions.margin.right, 190.0);
    // y = header height (60) + main margin-top (20) = 80.0
    assert_eq!(main.dimensions.border_box().y, 80.0);

    let footer = &body.children[2];
    assert_eq!(footer.dimensions.content.height, 40.0);
    // footer starts below main's margin box
    assert!(footer.dimensions.content.y >= main.dimensions.margin_box().y + main.dimensions.margin_box().height);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_layout --test integration_test`
Expected: FAIL

- [ ] **Step 3: Implement `layout_document` and root viewport initialization**
  - Construct viewport initial containing block.
  - Build layout tree via `build_layout_tree`.
  - Invoke `layout` on root.
  - Export `layout_document` in `nirvana_layout::lib`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_layout --test integration_test`
Expected: PASS

- [ ] **Step 5: Run full workspace test suite and linter**

Run: `cargo clippy --workspace` and `cargo test --workspace`
Expected: ALL PASS across all 6 crates (`nirvana_core`, `nirvana_url`, `nirvana_dom`, `nirvana_html`, `nirvana_css`, `nirvana_style`, `nirvana_layout`).

- [ ] **Step 6: Commit**

```bash
git add crates/nirvana_layout
git commit -m "feat(layout): implement full document layout pipeline with viewport resolution"
```
