# Phase 8: Interactive Input, Hit-Testing & Viewport Scrolling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the interactive input subsystem enabling touch pan/fling scrolling, document coordinate translation, front-to-back layout tree hit-testing, and link tap detection on Android.

**Architecture:** Extend `nirvana_layout` with front-to-back reverse-painting order hit-testing (`hit_test(point) -> Option<Id<Node>>`). Extend `nirvana_gfx` with offset display list rendering (`render_display_list_with_offset`). Extend `nirvana_android::NirvanaEngine` with viewport scroll state (`scroll_x`, `scroll_y`, bounds clamping, `hit_test(x, y)`), expose JNI scrolling and hit-test APIs, and integrate Android `GestureDetector` with kinetic `OverScroller` in `NirvanaView.kt`.

**Tech Stack:** Rust 2024 edition, Android `MotionEvent` / `GestureDetector` / `OverScroller`, JNI, Kotlin.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints

- Pure Rust with strict memory safety: `#![forbid(unsafe_code)]` at crate roots of core crates.
- JNI boundary safety: all exported C-ABI functions must wrap execution in `std::panic::catch_unwind`.
- Zero clippy warnings across `--workspace --all-targets`.
- Scroll clamping: `scroll_x` and `scroll_y` must never go below `0.0` or exceed `(document_dimension - viewport_dimension).max(0.0)`.

## Review Focus

1. **Hit-Testing Painting Order:** In CSS, elements painted on top must receive touch events first. Hit-testing must traverse children in **reverse** document order (last child first) to match visual stacking.
2. **Scroll Coordinate Transformation:** Converting between screen coordinates `(sx, sy)` and document coordinates `(dx, dy)` must correctly add `scroll_x` and `scroll_y`: `dx = sx + scroll_x`, `dy = sy + scroll_y`.
3. **Decoupled Scroll Re-Rendering:** Changing scroll position must NOT re-run expensive HTML parsing, CSS cascade, or layout tree construction; it must only re-rasterize the existing display list with the new viewport translation offset.
4. **Boundary Over-Scroll Prevention:** Negative delta scrolls or scrolling past the bottom of the page must clamp cleanly without wrapping, NaN values, or visual distortion.
5. **Anchor Tag Target Resolution:** Tapping an inner text or inline node inside an `<a href="...">` element must traverse ancestors to find the containing anchor tag and retrieve its `href` attribute.

---

### Task 1: Viewport Scroll State & Offset Display List Rasterization (`crates/nirvana_gfx` & `crates/nirvana_android`)

**Files:**
- Modify: `crates/nirvana_gfx/src/canvas.rs`
- Modify: `crates/nirvana_android/src/engine.rs`
- Test: `crates/nirvana_gfx/tests/scroll_rendering_test.rs`
- Test: `crates/nirvana_android/tests/scroll_test.rs`

**Interfaces:**
- Produces in `nirvana_gfx::canvas::Canvas`:
  - `Canvas::render_display_list_with_offset(&mut self, list: &DisplayList, offset_x: f32, offset_y: f32)`
- Produces in `nirvana_android::engine::NirvanaEngine`:
  - `NirvanaEngine::scroll_to(&mut self, x: f32, y: f32)`
  - `NirvanaEngine::scroll_by(&mut self, dx: f32, dy: f32)`
  - `NirvanaEngine::scroll_offset(&self) -> (f32, f32)`
  - `NirvanaEngine::document_dimensions(&self) -> (f32, f32)`
  - `NirvanaEngine::max_scroll(&self) -> (f32, f32)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_gfx/tests/scroll_rendering_test.rs` and `crates/nirvana_android/tests/scroll_test.rs`**

```rust
// In crates/nirvana_gfx/tests/scroll_rendering_test.rs
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_gfx::canvas::Canvas;
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};

#[test]
fn test_render_display_list_with_scroll_offset() {
    let mut canvas = Canvas::new(100, 100);
    let mut list = DisplayList::new();
    // A red box positioned at y = 50 in document coordinates
    list.push(DisplayCommand::SolidColor {
        rect: Rect::new(0.0, 50.0, 100.0, 50.0),
        color: Rgba::new(255, 0, 0, 255),
    });

    // When scrolled down by 50px (offset_y = -50.0), the box shifts to y = 0..50
    canvas.render_display_list_with_offset(&list, 0.0, -50.0);

    // Pixel at y = 10 should now be red
    assert_eq!(canvas.pixel_at(50, 10), Rgba::new(255, 0, 0, 255));
    // Pixel at y = 80 should be empty (0, 0, 0, 0)
    assert_eq!(canvas.pixel_at(50, 80), Rgba::new(0, 0, 0, 0));
}
```

```rust
// In crates/nirvana_android/tests/scroll_test.rs
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_scroll_clamping_and_bounds() {
    let mut engine = NirvanaEngine::new(400, 300);
    // Tall document of 1000px height
    let html = r#"<div style="height: 1000px; width: 400px; background-color: #ff0000;"></div>"#;
    engine.load_html(html, None);
    engine.render();

    assert_eq!(engine.scroll_offset(), (0.0, 0.0));
    let (max_x, max_y) = engine.max_scroll();
    assert_eq!(max_x, 0.0);
    assert_eq!(max_y, 700.0); // 1000 - 300

    // Scroll down 200px
    engine.scroll_by(0.0, 200.0);
    assert_eq!(engine.scroll_offset(), (0.0, 200.0));

    // Scroll past bottom: must clamp to max_y
    engine.scroll_by(0.0, 1000.0);
    assert_eq!(engine.scroll_offset(), (0.0, 700.0));

    // Scroll past top: must clamp to 0.0
    engine.scroll_by(0.0, -999.0);
    assert_eq!(engine.scroll_offset(), (0.0, 0.0));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nirvana_gfx --test scroll_rendering_test` and `cargo test -p nirvana_android --test scroll_test`
Expected: FAIL

- [ ] **Step 3: Implement offset rasterization and scroll state**
  - In `crates/nirvana_gfx/src/canvas.rs`:
    - Implement `render_display_list_with_offset(&mut self, list: &DisplayList, offset_x: f32, offset_y: f32)`.
    - Apply `(offset_x, offset_y)` translation to all `rect` and `origin` coordinates before rasterizing.
    - `render_display_list(&mut self, list)` simply delegates to `render_display_list_with_offset(list, 0.0, 0.0)`.
  - In `crates/nirvana_android/src/engine.rs`:
    - Add `scroll_x: f32`, `scroll_y: f32`, and `display_list: Option<DisplayList>` to `NirvanaEngine`.
    - Implement `scroll_to`, `scroll_by`, `scroll_offset`, `document_dimensions`, `max_scroll`.
    - In `scroll_by` / `scroll_to`, update offset, clamp to bounds, clear canvas, and re-rasterize existing `display_list` with `render_display_list_with_offset(-scroll_x, -scroll_y)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p nirvana_gfx --test scroll_rendering_test` and `cargo test -p nirvana_android --test scroll_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_gfx crates/nirvana_android
git commit -m "feat(scroll): implement viewport scroll state and offset display list rasterization"
```

---

### Task 2: Front-to-Back Layout Tree Hit-Testing (`hit_test.rs` in `crates/nirvana_layout`)

**Files:**
- Create: `crates/nirvana_layout/src/hit_test.rs`
- Modify: `crates/nirvana_layout/src/lib.rs`
- Test: `crates/nirvana_layout/tests/hit_test_test.rs`

**Interfaces:**
- Consumes: `LayoutBox`, `nirvana_core::geometry::Point`, `nirvana_dom::node::NodeId`
- Produces:
  - `LayoutBox::hit_test(&self, point: Point) -> Option<Id<Node>>`

- [ ] **Step 1: Write failing tests in `crates/nirvana_layout/tests/hit_test_test.rs`**

```rust
use nirvana_core::geometry::{Point, Size};
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_hit_test_nested_element() {
    let html = r#"
        <html>
            <body>
                <div id="container" style="width: 200px; height: 200px;">
                    <a id="link" href="https://example.com" style="width: 100px; height: 50px;">Click</a>
                </div>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).expect("root");

    // Hit test inside the <a> tag at (x: 20, y: 20)
    let hit_node = layout_root.hit_test(Point::new(20.0, 20.0));
    assert!(hit_node.is_some());
    let hit_id = hit_node.unwrap();
    // Verify hit node tag is 'a' or text child of 'a'
    assert!(dom.tag_name(hit_id) == Some("a") || dom.parent(hit_id).and_then(|p| dom.tag_name(p)) == Some("a"));

    // Hit test completely outside container at (x: 500, y: 500)
    let outside_hit = layout_root.hit_test(Point::new(500.0, 500.0));
    assert!(outside_hit.is_none() || outside_hit == layout_root.node_id());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_layout --test hit_test_test`
Expected: FAIL

- [ ] **Step 3: Implement `hit_test` in `crates/nirvana_layout/src/hit_test.rs`**
  - Implement reverse front-to-back traversal:
    1. Check if `self.dimensions.border_box().contains_point(point)`. If false, return `None`.
    2. Iterate `self.children` in **reverse order** (`self.children.iter().rev()`).
    3. If any child returns `Some(id)`, return `Some(id)`.
    4. If no children hit, return `self.node_id()`.
  - Export in `crates/nirvana_layout/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_layout --test hit_test_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_layout
git commit -m "feat(layout): implement front-to-back reverse painting order hit testing"
```

---

### Task 3: Hit-Test Target Information & JNI Bridge (`crates/nirvana_android`)

**Files:**
- Create: `crates/nirvana_android/src/hit_test.rs`
- Modify: `crates/nirvana_android/src/engine.rs`
- Modify: `crates/nirvana_android/src/jni_api.rs`
- Modify: `crates/nirvana_android/src/lib.rs`
- Test: `crates/nirvana_android/tests/engine_hit_test.rs`

**Interfaces:**
- Produces:
  - `pub struct HitTestResult { pub tag: String, pub href: Option<String>, pub text: Option<String> }`
  - `NirvanaEngine::hit_test(&self, screen_x: f32, screen_y: f32) -> Option<HitTestResult>`
  - JNI exports:
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeScrollBy(env, class, ptr, dx, dy)`
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeScrollTo(env, class, ptr, x, y)`
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetScrollX(env, class, ptr) -> jfloat`
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetScrollY(env, class, ptr) -> jfloat`
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetMaxScrollY(env, class, ptr) -> jfloat`
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeHitTest(env, class, ptr, x, y) -> jstring` (JSON string or empty string)

- [ ] **Step 1: Write failing test in `crates/nirvana_android/tests/engine_hit_test.rs`**

```rust
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_hit_test_detects_anchor_link_and_href() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<div style="padding: 20px;"><a href="https://nirvanabrowser.org" style="width: 100px; height: 30px;">Open Nirvana</a></div>"#;
    engine.load_html(html, None);
    engine.render();

    // Tap inside the link at (x: 25, y: 25)
    let result = engine.hit_test(25.0, 25.0);
    assert!(result.is_some());
    let hit = result.unwrap();
    assert_eq!(hit.tag, "a");
    assert_eq!(hit.href.as_deref(), Some("https://nirvanabrowser.org"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_android --test engine_hit_test`
Expected: FAIL

- [ ] **Step 3: Implement `HitTestResult`, `NirvanaEngine::hit_test`, and JNI bindings**
  - Implement `NirvanaEngine::hit_test`:
    - Converts `screen_x, screen_y` to `doc_x = screen_x + self.scroll_x`, `doc_y = screen_y + self.scroll_y`.
    - Calls `layout_root.hit_test(Point::new(doc_x, doc_y))`.
    - If a `NodeId` is returned: walks up parent chain until finding an `<a>` element (if any) or uses the clicked element.
    - Extracts `tag`, `href` attribute (if any), and text content.
  - Implement JNI exports in `jni_api.rs` with `catch_unwind` and null-pointer checks.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_android --test engine_hit_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_android
git commit -m "feat(android): implement hit test resolution and JNI scroll and hit test APIs"
```

---

### Task 4: Android Touch Gesture & Kinetic Fling Integration (`NirvanaView.kt`)

**Files:**
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaEngine.kt`
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaView.kt`

**Interfaces:**
- Produces in `NirvanaEngine.kt`:
  - `fun scrollBy(dx: Float, dy: Float)`
  - `fun scrollTo(x: Float, y: Float)`
  - `val scrollX: Float`
  - `val scrollY: Float`
  - `val maxScrollY: Float`
  - `fun hitTest(screenX: Float, screenY: Float): HitTarget?`
- Produces in `NirvanaView.kt`:
  - `interface OnLinkClickListener { fun onLinkClick(url: String) }`
  - `fun setOnLinkClickListener(listener: OnLinkClickListener?)`
  - `onTouchEvent(event: MotionEvent): Boolean` handling drag scroll, fling physics, and single tap link navigation.

- [ ] **Step 1: Add scroll and hit-test methods to `NirvanaEngine.kt`**
  - Add `nativeScrollBy`, `nativeScrollTo`, `nativeGetScrollX`, `nativeGetScrollY`, `nativeGetMaxScrollY`, `nativeHitTest`.
  - Create data class `HitTarget(val tag: String, val href: String?, val text: String?)`.

- [ ] **Step 2: Implement `GestureDetector` and `OverScroller` in `NirvanaView.kt`**
  - Instantiate `GestureDetector(context, gestureListener)` and `OverScroller(context)`.
  - In `onScroll`: call `engine.scrollBy(distanceX, distanceY)` and `drawFrame()`.
  - In `onFling`: call `scroller.fling(...)` and `postOnAnimation(flingRunnable)`.
  - In `onSingleTapUp`: call `engine.hitTest(e.x, e.y)`; if `href != null`, notify `onLinkClickListener?.onLinkClick(href)`.
  - In `computeScroll`: advance scroller and update scroll position smoothly.

- [ ] **Step 3: Commit**

```bash
git add android/app/src/main/kotlin
git commit -m "feat(android): implement touch gestures, kinetic fling scrolling, and link click detection in NirvanaView"
```

---

### Task 4: End-to-End Interactive Pipeline Test & Full Workspace Verification

**Files:**
- Create: `crates/nirvana_android/tests/interactive_e2e_test.rs`
- Modify: `progress.md`

- [ ] **Step 1: Write comprehensive interactive integration test**
  - Load a multi-section HTML document with header, paragraphs, and anchor links.
  - Test scrolling down shifts the visible pixel window.
  - Test hit testing links at initial and scrolled positions.

- [ ] **Step 2: Run test suite across workspace**

Run: `cargo test --workspace`
Expected: ALL PASS across all 10 crates.

- [ ] **Step 3: Run Clippy linter**

Run: `cargo clippy --workspace --all-targets`
Expected: 0 warnings.

- [ ] **Step 4: Commit and update progress ledger**

```bash
git add progress.md crates/nirvana_android/tests/interactive_e2e_test.rs
git commit -m "test(android): add end-to-end interactive scrolling and hit testing integration test"
```
