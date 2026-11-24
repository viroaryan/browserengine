# Phase 7: Typography & Font Rasterization Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `nirvana_font` providing font loading, glyph rasterization, and text measurement, and integrate it into `nirvana_layout` and `nirvana_gfx` to replace placeholder text markers with real, anti-aliased typography.

**Architecture:** A new pure-Rust crate `nirvana_font` powered by `fontdue`. It loads TrueType/OpenType font bytes, computes string dimensions and line metrics, rasterizes glyphs into 8-bit alpha coverage masks, and blends glyphs onto `nirvana_gfx::Canvas`. `nirvana_layout` uses font metrics so block and inline elements calculate accurate text content sizes.

**Tech Stack:** Rust 2024 edition, `fontdue = "0.9.5"`, `nirvana_core`, `nirvana_gfx`, `nirvana_layout`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints

- Pure Rust with strict memory safety: `#![forbid(unsafe_code)]` at crate root.
- Zero clippy warnings across `--workspace --all-targets`.
- Embedded default fallback font: The engine must include a default TrueType font embedded via `include_bytes!` so that rendering works offline and cross-platform without depending on host OS font directories.
- Anti-aliased subpixel/coverage alpha blending: Glyphs must blend seamlessly onto arbitrary background colors using Porter-Duff Source Over alpha blending.

## Review Focus

1. **Zero-Width & Whitespace Handling:** Strings with spaces, newlines, tabs, or non-printable characters must compute accurate advance widths without crashing or panic.
2. **Font Size Scaling:** Font metrics and glyph dimensions must scale proportionally with `font_size_px`.
3. **Clipping & Canvas Bounds:** Glyphs crossing canvas boundaries (e.g. partially off-screen text) must clamp coordinates and avoid out-of-bounds buffer writes.
4. **Layout Height Accumulation:** Block containers with `height: auto` wrapping text children must now reflect the real text line height instead of collapsing to 0.
5. **Unicode Coverage & Missing Glyphs:** Missing glyphs (not in font table) must fall back to the `.notdef` replacement glyph cleanly without panic.

---

### Task 1: `nirvana_font` Crate Scaffold & Font Loading (`font.rs`)

**Files:**
- Create: `crates/nirvana_font/Cargo.toml`
- Create: `crates/nirvana_font/src/lib.rs`
- Create: `crates/nirvana_font/src/font.rs`
- Create: `crates/nirvana_font/assets/default_font.ttf`
- Modify: `Cargo.toml` (add `crates/nirvana_font` to workspace)
- Test: `crates/nirvana_font/tests/font_test.rs`

**Interfaces:**
- Consumes: `fontdue::Font`
- Produces:
  - `pub struct Font`
  - `Font::from_bytes(data: &[u8]) -> Result<Self, &'static str>`
  - `Font::default_font() -> &'static Font`

- [ ] **Step 1: Write failing tests in `crates/nirvana_font/tests/font_test.rs`**

```rust
use nirvana_font::font::Font;

#[test]
fn test_default_font_loads_successfully() {
    let font = Font::default_font();
    assert!(font.has_glyph('A'));
    assert!(font.has_glyph('z'));
    assert!(font.has_glyph('0'));
    assert!(font.has_glyph(' '));
}

#[test]
fn test_invalid_font_bytes_return_error() {
    let bad_bytes = b"not a true type font";
    let result = Font::from_bytes(bad_bytes);
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_font --test font_test`
Expected: FAIL (crate does not exist yet)

- [ ] **Step 3: Implement `Font` in `crates/nirvana_font/src/font.rs`**
  - Add `fontdue = "0.9.5"` to `crates/nirvana_font/Cargo.toml`.
  - Embed a minimal TrueType font in `assets/default_font.ttf`.
  - Wrap `fontdue::Font` in `Font`.
  - Implement `from_bytes` and `default_font` (using `std::sync::OnceLock`).
  - Implement `has_glyph(c: char) -> bool`.
  - Export in `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_font --test font_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_font
git commit -m "feat(font): scaffold nirvana_font crate and font loading abstractions"
```

---

### Task 2: Text Measurement & Line Metrics (`metrics.rs`)

**Files:**
- Create: `crates/nirvana_font/src/metrics.rs`
- Modify: `crates/nirvana_font/src/lib.rs`
- Test: `crates/nirvana_font/tests/metrics_test.rs`

**Interfaces:**
- Consumes: `Font`
- Produces:
  - `pub struct TextMetrics { pub width: f32, pub height: f32, pub ascent: f32, pub descent: f32, pub line_height: f32 }`
  - `Font::measure_text(&self, text: &str, font_size_px: f32) -> TextMetrics`
  - `Font::line_height(&self, font_size_px: f32) -> f32`

- [ ] **Step 1: Write failing tests in `crates/nirvana_font/tests/metrics_test.rs`**

```rust
use nirvana_font::font::Font;

#[test]
fn test_measure_text_dimensions() {
    let font = Font::default_font();
    let m1 = font.measure_text("Hello", 16.0);
    let m2 = font.measure_text("Hello World", 16.0);

    assert!(m1.width > 0.0);
    assert!(m2.width > m1.width, "Longer text must have larger advance width");
    assert!(m1.height > 0.0);
    assert!(m1.line_height >= 16.0);
}

#[test]
fn test_font_size_scaling() {
    let font = Font::default_font();
    let m16 = font.measure_text("Test", 16.0);
    let m32 = font.measure_text("Test", 32.0);

    // Twice the font size should be approximately twice the width (within 10%)
    let ratio = m32.width / m16.width;
    assert!((ratio - 2.0).abs() < 0.2, "Width should scale linearly with font size");
}

#[test]
fn test_empty_string_measurement() {
    let font = Font::default_font();
    let m = font.measure_text("", 16.0);
    assert_eq!(m.width, 0.0);
    assert!(m.line_height > 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_font --test metrics_test`
Expected: FAIL

- [ ] **Step 3: Implement `measure_text` and `metrics.rs`**
  - Iterate characters in `text`, querying `self.inner.metrics(c, font_size_px).advance_width`.
  - Retrieve vertical line metrics via `self.inner.horizontal_line_metrics(font_size_px)`.
  - Calculate `line_height = ascent - descent + line_gap`.
  - Export in `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_font --test metrics_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_font/src/metrics.rs crates/nirvana_font/tests/metrics_test.rs
git commit -m "feat(font): implement text measurement and line metrics calculation"
```

---

### Task 3: Glyph Rasterization & Anti-Aliased Text Blitting (`rasterizer.rs`)

**Files:**
- Create: `crates/nirvana_font/src/rasterizer.rs`
- Modify: `crates/nirvana_font/src/lib.rs`
- Test: `crates/nirvana_font/tests/rasterizer_test.rs`

**Interfaces:**
- Consumes: `Font`, `nirvana_gfx::Canvas`, `nirvana_core::color::Rgba`
- Produces:
  - `pub struct GlyphBitmap { pub width: usize, pub height: usize, pub xmin: i32, pub ymin: i32, pub alpha_mask: Vec<u8> }`
  - `Font::rasterize_glyph(&self, c: char, font_size_px: f32) -> GlyphBitmap`
  - `nirvana_font::draw_text(canvas: &mut Canvas, font: &Font, text: &str, x: f32, y: f32, font_size_px: f32, color: Rgba)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_font/tests/rasterizer_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_gfx::canvas::Canvas;
use nirvana_font::font::Font;
use nirvana_font::draw_text;

#[test]
fn test_rasterize_single_glyph() {
    let font = Font::default_font();
    let glyph = font.rasterize_glyph('A', 24.0);

    assert!(glyph.width > 0);
    assert!(glyph.height > 0);
    assert_eq!(glyph.alpha_mask.len(), glyph.width * glyph.height);
    // At least some pixels must have non-zero alpha coverage
    assert!(glyph.alpha_mask.iter().any(|&a| a > 0));
}

#[test]
fn test_draw_text_onto_canvas() {
    let mut canvas = Canvas::new(200, 100);
    let font = Font::default_font();
    let red = Rgba::new(255, 0, 0, 255);

    draw_text(&mut canvas, font, "Hi", 10.0, 30.0, 20.0, red);

    // Verify some red pixels were painted on the canvas
    let has_red_pixel = (0..100).any(|y| {
        (0..200).any(|x| {
            let px = canvas.pixel_at(x, y);
            px.r > 200 && px.a > 0
        })
    });
    assert!(has_red_pixel, "Canvas must contain rendered red glyph pixels");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_font --test rasterizer_test`
Expected: FAIL

- [ ] **Step 3: Implement `rasterizer.rs`**
  - Implement `rasterize_glyph` using `fontdue::Font::rasterize(c, font_size_px)`.
  - Implement `draw_text`:
    - Tracks cursor X, advancing by `advance_width`.
    - Rasterizes each character.
    - Blends each pixel of the glyph alpha mask onto `Canvas` using Porter-Duff Source Over with `alpha = (color.a as f32 * mask_a as f32 / 255.0) as u8`.
    - Clamps to canvas bounds to prevent out-of-bounds writes.
  - Export in `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_font --test rasterizer_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_font/src/rasterizer.rs crates/nirvana_font/tests/rasterizer_test.rs
git commit -m "feat(font): implement glyph rasterization and anti-aliased text drawing"
```

---

### Task 4: Layout Engine Inline Box Text Sizing (`nirvana_layout`)

**Files:**
- Modify: `crates/nirvana_layout/Cargo.toml` (add `nirvana_font`)
- Modify: `crates/nirvana_layout/src/block.rs`
- Test: `crates/nirvana_layout/tests/text_layout_test.rs`

**Interfaces:**
- Consumes: `nirvana_font::Font::default_font()`, `measure_text`
- Modifies: `calculate_block_height` and `layout_inline` in `nirvana_layout`

- [ ] **Step 1: Write failing test in `crates/nirvana_layout/tests/text_layout_test.rs`**

```rust
use nirvana_core::geometry::Size;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_block_height_expands_to_fit_text_content() {
    let html = r#"<p>Some actual text content</p>"#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).expect("layout root");

    // The <p> element contains text; its content height must be > 0.0 (approximately 16px line height)
    let p_box = &layout_root.children[0]; // body -> p
    assert!(
        p_box.dimensions.content.height() >= 12.0,
        "Block container height must expand to accommodate text height, was {}",
        p_box.dimensions.content.height()
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_layout --test text_layout_test`
Expected: FAIL (height currently evaluates to 0.0)

- [ ] **Step 3: Update `nirvana_layout` text measurement**
  - In `layout_inline`: When a layout box is `BoxType::InlineNode(id)` for a `NodeType::Text`, measure the string with `nirvana_font::Font::default_font().measure_text(trimmed, 16.0)`.
  - Set `self.dimensions.content.size = Size::new(metrics.width, metrics.line_height)`.
  - Parent block `calculate_block_height` will automatically sum the child's non-zero height!

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_layout --test text_layout_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_layout
git commit -m "feat(layout): compute real text dimensions and line heights in inline layout"
```

---

### Task 5: End-to-End Visual Text Rendering Pipeline & Workspace Verification

**Files:**
- Modify: `crates/nirvana_gfx/Cargo.toml` (add `nirvana_font`)
- Modify: `crates/nirvana_gfx/src/canvas.rs` (render real glyphs in `render_display_list`)
- Test: `crates/nirvana_gfx/tests/text_rendering_e2e_test.rs`
- Modify: `progress.md`

- [ ] **Step 1: Write failing test in `crates/nirvana_gfx/tests/text_rendering_e2e_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Size;
use nirvana_css::parser::parse_css;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_html_text_renders_actual_glyphs_into_canvas() {
    let html = r#"
        <div style="background-color: #ffffff; padding: 20px;">
            <p style="color: #ff0000;">Hello Nirvana</p>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(400.0, 200.0)).unwrap();
    let display_list = build_display_list(&layout_root, &style_tree, &dom);

    let mut canvas = Canvas::new(400, 200);
    canvas.render_display_list(&display_list);

    // Verify canvas contains anti-aliased red text pixels
    let mut red_pixel_count = 0;
    for y in 0..200 {
        for x in 0..400 {
            let px = canvas.pixel_at(x, y);
            if px.r > 200 && px.g < 50 && px.b < 50 {
                red_pixel_count += 1;
            }
        }
    }

    assert!(
        red_pixel_count > 50,
        "Expected multiple red glyph pixels from rendered text, found {}",
        red_pixel_count
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_gfx --test text_rendering_e2e_test`
Expected: FAIL (only 16 pixels from 4x4 placeholder dot)

- [ ] **Step 3: Update `render_display_list` in `crates/nirvana_gfx/src/canvas.rs`**
  - Replace the 4x4 placeholder rectangle with `nirvana_font::draw_text(&mut self, font, text, origin.x, origin.y, 16.0, *color)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_gfx --test text_rendering_e2e_test`
Expected: PASS

- [ ] **Step 5: Run full workspace test suite and linter**

Run: `cargo test --workspace` and `cargo clippy --workspace --all-targets`
Expected: ALL PASS with 0 warnings across all 10 crates.

- [ ] **Step 6: Commit and update progress ledger**

```bash
git add progress.md crates/nirvana_gfx
git commit -m "feat(gfx): render anti-aliased font glyphs in display list execution"
```
