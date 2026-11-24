# Nirvana Engine Phase 5: 2D Painting & Rasterization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a 2D painting and rasterization engine (`nirvana_gfx`) from scratch, transforming a positioned `LayoutBox` tree into a display list of drawing commands, and rasterizing them into a 32-bit RGBA pixel framebuffer (Canvas) with alpha blending and PNG visual snapshot generation.

**Architecture:** A three-stage rendering pipeline:
1. `display_list`: Intermediate representation (`DisplayCommand`, `DisplayList`) abstracting painting operations (`SolidColor`, `Border`, `Text`).
2. `builder`: CSS 2.1 Appendix E painting order traversal producing a `DisplayList` from `(LayoutBox, StyleTree, DomTree)` (Background -> Borders -> Children -> Text).
3. `canvas`: High-performance 2D software framebuffer supporting Porter-Duff alpha blending, boundary clipping, integer rasterization, and PNG image export (integrated with `tiny-skia` for PNG encoding and Android SurfaceView raw pixel buffer compatibility).

**Tech Stack:** Pure Rust (Edition 2024 / 1.85+ compatible, tested on Rust 1.99), `nirvana_core`, `nirvana_dom`, `nirvana_style`, `nirvana_layout`, `tiny-skia` (for PNG encoding), `#![forbid(unsafe_code)]`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints
- **Language & Safety:** Pure Rust with `#![forbid(unsafe_code)]` in `nirvana_gfx`.
- **From Scratch Display List:** Display list commands and software rasterization arithmetic (coordinates, clipping, alpha blending) written from foundational principles.
- **Color Model:** 32-bit RGBA (`[r, g, b, a]`, 8-bits per channel, 4 bytes per pixel) compatible with Android `ANativeWindow` buffer formats (`WINDOW_FORMAT_RGBA_8888`).
- **Coordinate System:** Pixel grid matching layout tree coordinates (origin at top-left $(0, 0)$, positive X right, positive Y down).

## Review Focus
1. **CSS Painting Order:** Elements must paint backgrounds first, then borders, then children, then text. If child A is followed by child B, child B paints on top of child A.
2. **Alpha Blending:** Transparent / semi-transparent colors must blend cleanly using standard Porter-Duff over operator:
   $$\text{out} = \text{src} \times \alpha_{\text{src}} + \text{dst} \times (1 - \alpha_{\text{src}})$$
   Fully transparent backgrounds (`alpha == 0`) must generate no op or preserve destination pixels.
3. **Viewport Clipping:** Drawing commands extending outside canvas dimensions (negative coordinates or coordinates $> \text{width}/\text{height}$) must clamp safely without panicking or buffer overruns.
4. **Border Geometry:** Non-zero borders must paint on the outer perimeter of the border box without overflowing or creating hollow corners.
5. **Layout Pruning Conformance:** Hidden elements (`display: none`) and zero-sized elements must not emit unnecessary display commands.

---

### Task 1: `nirvana_gfx` Crate Setup & Display List AST

**Files:**
- Modify: `Cargo.toml:4-13`
- Create: `crates/nirvana_gfx/Cargo.toml`
- Create: `crates/nirvana_gfx/src/lib.rs`
- Create: `crates/nirvana_gfx/src/display_list.rs`
- Test: `crates/nirvana_gfx/tests/display_list_test.rs`

**Interfaces:**
- Consumes: `nirvana_core::color::Rgba`, `nirvana_core::geometry::{Point, Rect}`, `nirvana_layout::dimensions::EdgeSizes`
- Produces:
  - `nirvana_gfx::display_list::{DisplayCommand, DisplayList}`
  - `DisplayList::new() -> Self`
  - `DisplayList::push(&mut self, command: DisplayCommand)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_gfx/tests/display_list_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_core::geometry::{Point, Rect};
use nirvana_layout::dimensions::EdgeSizes;
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};

#[test]
fn test_display_list_commands() {
    let mut list = DisplayList::new();
    list.push(DisplayCommand::SolidColor {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        color: Rgba::new(255, 0, 0, 255),
    });
    list.push(DisplayCommand::Border {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        widths: EdgeSizes::new(2.0, 2.0, 2.0, 2.0),
        color: Rgba::new(0, 0, 0, 255),
    });
    list.push(DisplayCommand::Text {
        text: "Hello Nirvana".into(),
        origin: Point::new(10.0, 30.0),
        color: Rgba::new(255, 255, 255, 255),
    });

    assert_eq!(list.commands.len(), 3);
    assert!(matches!(&list.commands[0], DisplayCommand::SolidColor { .. }));
    assert!(matches!(&list.commands[1], DisplayCommand::Border { .. }));
    assert!(matches!(&list.commands[2], DisplayCommand::Text { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_gfx --test display_list_test`
Expected: FAIL (crate not found)

- [ ] **Step 3: Implement `nirvana_gfx` crate and `display_list.rs`**
  - Add `"crates/nirvana_gfx"` to root `Cargo.toml`.
  - Create `crates/nirvana_gfx/Cargo.toml` with `nirvana_core`, `nirvana_layout`, `nirvana_dom`, `nirvana_style`, `tiny-skia = "0.12.0"`.
  - Implement `DisplayCommand` enum and `DisplayList` struct.
  - Export in `crates/nirvana_gfx/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_gfx --test display_list_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_gfx
git commit -m "feat(gfx): scaffold nirvana_gfx crate and display list AST"
```

---

### Task 2: Display List Builder (CSS 2.1 Painting Order)

**Files:**
- Create: `crates/nirvana_gfx/src/builder.rs`
- Test: `crates/nirvana_gfx/tests/builder_test.rs`

**Interfaces:**
- Consumes: `nirvana_layout::box_type::LayoutBox`, `nirvana_style::compute::StyleTree`, `nirvana_dom::DomTree`
- Produces:
  - `nirvana_gfx::builder::build_display_list(layout_root: &LayoutBox, style_tree: &StyleTree, dom: &DomTree) -> DisplayList`

- [ ] **Step 1: Write failing tests in `crates/nirvana_gfx/tests/builder_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Size;
use nirvana_html::parse_html;
use nirvana_css::parser::parse_css;
use nirvana_style::compute::compute_styles;
use nirvana_layout::layout::layout_document;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::display_list::DisplayCommand;

#[test]
fn test_build_display_list_painting_order() {
    let html = r#"
        <div style="background-color: #ff0000; border: 2px; padding: 10px;">
            <p style="background-color: #00ff00;">Text content</p>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let display_list = build_display_list(&layout_root, &style_tree, &dom);

    // Filter commands by color
    let red = Rgba::new(255, 0, 0, 255);
    let green = Rgba::new(0, 255, 0, 255);

    // Parent background must precede child background in the display list
    let red_idx = display_list.commands.iter().position(|cmd| {
        matches!(cmd, DisplayCommand::SolidColor { color, .. } if *color == red)
    }).expect("parent red background emitted");

    let green_idx = display_list.commands.iter().position(|cmd| {
        matches!(cmd, DisplayCommand::SolidColor { color, .. } if *color == green)
    }).expect("child green background emitted");

    assert!(red_idx < green_idx, "Parent background must be painted before child background");

    // Text content must be emitted
    assert!(display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Text { text, .. } if text.contains("Text content"))
    }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_gfx --test builder_test`
Expected: FAIL

- [ ] **Step 3: Implement `build_display_list`**
  - For each `LayoutBox` recursively in tree order:
    1. Look up computed style.
    2. Background: If `background_color.a > 0`, push `SolidColor { rect: padding_box, color: background_color }`.
    3. Borders: If `border_width > 0` and `border.left > 0`, push `Border { rect: border_box, widths: border, color }`.
    4. Text: If `box_type` is an `InlineNode` backing a `NodeType::Text`, extract string and push `DisplayCommand::Text`.
    5. Recurse for all children.
  - Export in `crates/nirvana_gfx/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_gfx --test builder_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_gfx
git commit -m "feat(gfx): implement display list builder following CSS painting order"
```

---

### Task 3: 2D Framebuffer & Software Rasterizer (`canvas.rs`)

**Files:**
- Create: `crates/nirvana_gfx/src/canvas.rs`
- Test: `crates/nirvana_gfx/tests/rasterizer_test.rs`

**Interfaces:**
- Consumes: `nirvana_gfx::display_list::*`, `nirvana_core::color::Rgba`, `nirvana_core::geometry::Rect`
- Produces:
  - `nirvana_gfx::canvas::Canvas`
  - `Canvas::new(width: u32, height: u32) -> Self`
  - `Canvas::pixel_at(&self, x: u32, y: u32) -> Rgba`
  - `Canvas::fill_rect(&mut self, rect: Rect, color: Rgba)`
  - `Canvas::draw_borders(&mut self, rect: Rect, widths: EdgeSizes, color: Rgba)`
  - `Canvas::render_display_list(&mut self, list: &DisplayList)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_gfx/tests/rasterizer_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_layout::dimensions::EdgeSizes;
use nirvana_gfx::canvas::Canvas;
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};

#[test]
fn test_canvas_solid_color_and_pixel_lookup() {
    let mut canvas = Canvas::new(100, 100);
    // Fill background with white
    canvas.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Rgba::new(255, 255, 255, 255));

    // Fill small red box in the middle (x: 20..60, y: 30..70)
    canvas.fill_rect(Rect::new(20.0, 30.0, 40.0, 40.0), Rgba::new(255, 0, 0, 255));

    assert_eq!(canvas.pixel_at(0, 0), Rgba::new(255, 255, 255, 255));
    assert_eq!(canvas.pixel_at(30, 40), Rgba::new(255, 0, 0, 255));
    assert_eq!(canvas.pixel_at(70, 70), Rgba::new(255, 255, 255, 255));
}

#[test]
fn test_canvas_alpha_blending() {
    let mut canvas = Canvas::new(10, 10);
    // Background: opaque black
    canvas.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Rgba::new(0, 0, 0, 255));

    // Foreground: 50% transparent white (rgba: 255, 255, 255, 128)
    canvas.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Rgba::new(255, 255, 255, 128));

    let pixel = canvas.pixel_at(5, 5);
    // Blended: 0.5 * 255 = ~127 or 128
    assert!((pixel.r as i32 - 128).abs() <= 2);
    assert!((pixel.g as i32 - 128).abs() <= 2);
    assert!((pixel.b as i32 - 128).abs() <= 2);
    assert_eq!(pixel.a, 255);
}

#[test]
fn test_canvas_clipping_no_panic() {
    let mut canvas = Canvas::new(50, 50);
    // Draw box far outside canvas boundaries
    canvas.fill_rect(Rect::new(-100.0, -100.0, 200.0, 200.0), Rgba::new(255, 0, 0, 255));
    assert_eq!(canvas.pixel_at(10, 10), Rgba::new(255, 0, 0, 255));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_gfx --test rasterizer_test`
Expected: FAIL

- [ ] **Step 3: Implement `Canvas` software framebuffer**
  - In `crates/nirvana_gfx/src/canvas.rs`:
    - `pixels: Vec<u8>` in 32-bit RGBA (size `width * height * 4`).
    - Porter-Duff alpha blending helper.
    - Clamped integer bounds calculation for `Rect`.
    - `fill_rect` rasterizer with alpha blending.
    - `draw_borders` rasterizing 4 edge rects (top, right, bottom, left).
    - `render_display_list` iterating through commands.
  - Export in `crates/nirvana_gfx/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_gfx --test rasterizer_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_gfx
git commit -m "feat(gfx): implement 2d software rasterizer and canvas framebuffer"
```

---

### Task 4: PNG Export & End-to-End Visual Integration Test

**Files:**
- Create: `crates/nirvana_gfx/tests/e2e_render_test.rs`
- Modify: `crates/nirvana_gfx/src/canvas.rs`
- Modify: `crates/nirvana_gfx/src/lib.rs`

**Interfaces:**
- Produces:
  - `Canvas::save_png(&self, path: &std::path::Path) -> Result<(), std::io::Error>`
  - `Canvas::encode_png(&self) -> Result<Vec<u8>, std::io::Error>`

- [ ] **Step 1: Write integration tests in `crates/nirvana_gfx/tests/e2e_render_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Size;
use nirvana_html::parse_html;
use nirvana_css::parser::parse_css;
use nirvana_style::compute::compute_styles;
use nirvana_layout::layout::layout_document;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;

#[test]
fn test_end_to_end_html_css_to_pixel_buffer() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { margin: 0; background-color: #ffffff; }
                    header { height: 50px; background-color: #0000ff; }
                    main { width: 200px; height: 100px; margin: 20px auto; background-color: #ff0000; }
                </style>
            </head>
            <body>
                <header></header>
                <main></main>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let css = r#"
        body { margin: 0; background-color: #ffffff; }
        header { height: 50px; background-color: #0000ff; }
        main { width: 200px; height: 100px; margin: 20px auto; background-color: #ff0000; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let viewport = Size::new(400.0, 300.0);
    let layout_root = layout_document(&dom, &style_tree, viewport).expect("layout root");
    let display_list = build_display_list(&layout_root, &style_tree, &dom);

    let mut canvas = Canvas::new(400, 300);
    canvas.render_display_list(&display_list);

    // Verify header pixel (x: 200, y: 25) is blue (#0000ff)
    assert_eq!(canvas.pixel_at(200, 25), Rgba::new(0, 0, 255, 255));

    // Verify main centered box (width: 200, in 400 viewport -> x: 100..300, y: 70..170) is red (#ff0000)
    assert_eq!(canvas.pixel_at(200, 120), Rgba::new(255, 0, 0, 255));

    // Verify outside margin area (x: 50, y: 120) is body white (#ffffff)
    assert_eq!(canvas.pixel_at(50, 120), Rgba::new(255, 255, 255, 255));

    // Verify PNG encoding works
    let png_bytes = canvas.encode_png().expect("PNG encoded");
    assert!(!png_bytes.is_empty());
    assert_eq!(&png_bytes[0..8], b"\x89PNG\r\n\x1a\n");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_gfx --test e2e_render_test`
Expected: FAIL

- [ ] **Step 3: Implement PNG encoding with `tiny-skia`**
  - Implement `Canvas::encode_png(&self)` using `tiny-skia::PixmapRef::from_bytes(&self.pixels, self.width, self.height).unwrap().encode_png()`.
  - Implement `Canvas::save_png(&self, path: &std::path::Path)`.
  - Export in `crates/nirvana_gfx/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_gfx --test e2e_render_test`
Expected: PASS

- [ ] **Step 5: Run full workspace test suite and linter**

Run: `cargo clippy --workspace --all-targets` and `cargo test --workspace`
Expected: ALL PASS with 0 warnings across all 8 crates.

- [ ] **Step 6: Commit**

```bash
git add crates/nirvana_gfx
git commit -m "feat(gfx): implement png encoding and end-to-end visual rendering pipeline"
```
