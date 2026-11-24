# Phase 6: Android Integration & JNI Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `nirvana_android` providing an embeddable Android dynamic library (`libnirvana_engine.so`), JNI bindings, and surface rendering pipeline to host Nirvana Browser Engine inside an Android `SurfaceView`.

**Architecture:** A dedicated `nirvana_android` crate compiled as `cdylib` and `rlib`. It encapsulates `NirvanaEngine` (orchestrating DOM, CSS, layout, and software rasterizer), exposes JNI C-ABI exported functions (`nativeCreate`, `nativeDestroy`, `nativeLoadHtml`, `nativeResize`, `nativeRender`, `nativeGetPixels`, `nativeRenderToSurface`), and provides idiomatic Kotlin wrappers (`NirvanaEngine.kt`, `NirvanaView.kt`) for direct Android app embedding.

**Tech Stack:** Rust 2024 edition, `jni = "0.21"`, `tiny-skia`, Android NDK C-ABI / `ANativeWindow`, Kotlin/Android SDK bindings.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints

- Pure Rust modular design with clean separation of concerns.
- `#![forbid(unsafe_code)]` on pure engine logic; strictly isolate any JNI pointer dereferencing or FFI boundary code with documented safety invariants and `std::panic::catch_unwind` guards.
- Zero clippy warnings across `--workspace --all-targets`.
- Host testability: All engine and JNI buffer blitting logic must compile and pass tests on Windows/Linux host platforms without requiring an attached Android device.

## Review Focus

1. **JNI Pointer Safety & Double-Free Protection:** `nativeDestroy` and subsequent calls must not crash if given an invalid, null, or previously freed pointer.
2. **Panic Boundary Isolation:** Rust panics across JNI C-ABI boundaries cause undefined behavior / aborts in ART (Android Runtime); all JNI entry points must be wrapped in `std::panic::catch_unwind`.
3. **Viewport Resizing & Re-Layout:** Changing viewport dimensions must invalidate previous layout geometry and resize the framebuffer canvas without leaking memory.
4. **Buffer Stride & Pitch Handling:** Android surfaces frequently have row strides (stride in pixels) larger than width (`stride >= width`); blitting must handle stride padding without pixel shearing or memory corruption.
5. **String Encoding Safety:** Java UTF-16 / MUTF-8 strings passed via JNI must be cleanly decoded without truncation or panic on invalid Unicode.

---

### Task 1: `nirvana_android` Crate Setup & Engine State Machine (`engine.rs`)

**Files:**
- Create: `crates/nirvana_android/Cargo.toml`
- Create: `crates/nirvana_android/src/lib.rs`
- Create: `crates/nirvana_android/src/engine.rs`
- Modify: `Cargo.toml` (add `crates/nirvana_android` to workspace)
- Test: `crates/nirvana_android/tests/engine_test.rs`

**Interfaces:**
- Consumes:
  - `nirvana_html::parse_html(html: &str) -> DomTree`
  - `nirvana_css::parser::parse_css(css: &str) -> StyleSheet`
  - `nirvana_style::compute::compute_styles(dom: &DomTree, sheets: &[&StyleSheet]) -> StyleTree`
  - `nirvana_layout::layout::layout_document(dom: &DomTree, style_tree: &StyleTree, viewport: Size) -> Option<LayoutBox>`
  - `nirvana_gfx::builder::build_display_list(layout_root: &LayoutBox, style_tree: &StyleTree, dom: &DomTree) -> DisplayList`
  - `nirvana_gfx::canvas::Canvas`
- Produces:
  - `pub struct NirvanaEngine`
  - `NirvanaEngine::new(width: u32, height: u32) -> Self`
  - `NirvanaEngine::load_html(&mut self, html: &str, css: Option<&str>)`
  - `NirvanaEngine::resize(&mut self, width: u32, height: u32)`
  - `NirvanaEngine::render(&mut self)`
  - `NirvanaEngine::pixels(&self) -> &[u8]`
  - `NirvanaEngine::dimensions(&self) -> (u32, u32)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_android/tests/engine_test.rs`**

```rust
use nirvana_android::engine::NirvanaEngine;

#[test]
fn test_engine_lifecycle_and_rendering() {
    let mut engine = NirvanaEngine::new(400, 300);
    assert_eq!(engine.dimensions(), (400, 300));
    assert_eq!(engine.pixels().len(), 400 * 300 * 4);

    let html = r#"<div style="background-color: #ff0000; width: 100px; height: 100px;"></div>"#;
    engine.load_html(html, None);
    engine.render();

    // Verify non-empty pixel data was rendered
    let pixels = engine.pixels();
    assert_eq!(pixels.len(), 400 * 300 * 4);
    // At coordinate (10, 10), red color should be present: R=255, G=0, B=0, A=255
    let idx = (10 * 400 + 10) * 4;
    assert_eq!(pixels[idx], 255);
    assert_eq!(pixels[idx + 1], 0);
    assert_eq!(pixels[idx + 2], 0);
    assert_eq!(pixels[idx + 3], 255);
}

#[test]
fn test_engine_resize() {
    let mut engine = NirvanaEngine::new(200, 200);
    assert_eq!(engine.dimensions(), (200, 200));

    engine.resize(500, 400);
    assert_eq!(engine.dimensions(), (500, 400));
    assert_eq!(engine.pixels().len(), 500 * 400 * 4);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_android --test engine_test`
Expected: FAIL (crate does not exist yet)

- [ ] **Step 3: Implement `NirvanaEngine` and crate configuration**
  - Register `crates/nirvana_android` in workspace `Cargo.toml`.
  - Configure `crates/nirvana_android/Cargo.toml` with `crate-type = ["cdylib", "rlib"]`, dependencies on workspace crates and `jni = "0.21"`.
  - Implement `NirvanaEngine` managing `width`, `height`, `dom`, `stylesheets`, `style_tree`, `layout_root`, and `canvas`.
  - Implement `render()` executing the complete styling -> layout -> display list -> rasterization pipeline.
  - Export in `crates/nirvana_android/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_android --test engine_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/nirvana_android
git commit -m "feat(android): scaffold nirvana_android crate and NirvanaEngine state machine"
```

---

### Task 2: JNI Export Layer & Memory-Safe Pointer Lifecycle (`jni_api.rs`)

**Files:**
- Create: `crates/nirvana_android/src/jni_api.rs`
- Modify: `crates/nirvana_android/src/lib.rs`
- Test: `crates/nirvana_android/tests/jni_api_test.rs`

**Interfaces:**
- Consumes: `NirvanaEngine`
- Produces JNI C-ABI symbols:
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeCreate(env, class, width, height) -> jlong`
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeDestroy(env, class, engine_ptr)`
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeLoadHtml(env, class, engine_ptr, html, css)`
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeResize(env, class, engine_ptr, width, height)`
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeRender(env, class, engine_ptr)`
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetPixels(env, class, engine_ptr) -> jbyteArray`

- [ ] **Step 1: Write tests for pointer lifecycle and panic safety in `crates/nirvana_android/tests/jni_api_test.rs`**

```rust
use nirvana_android::engine::NirvanaEngine;
use std::sync::atomic::{AtomicPtr, Ordering};

#[test]
fn test_engine_heap_pointer_lifecycle() {
    let engine = Box::new(NirvanaEngine::new(300, 300));
    let raw_ptr = Box::into_raw(engine);

    assert!(!raw_ptr.is_null());

    // Verify pointer can be safely dereferenced
    unsafe {
        let engine_ref = &mut *raw_ptr;
        assert_eq!(engine_ref.dimensions(), (300, 300));
    }

    // Free heap memory
    unsafe {
        let _ = Box::from_raw(raw_ptr);
    }
}
```

- [ ] **Step 2: Run test to verify it passes/fails**

Run: `cargo test -p nirvana_android --test jni_api_test`
Expected: PASS (verifying heap pointer safety foundations)

- [ ] **Step 3: Implement JNI entry points in `crates/nirvana_android/src/jni_api.rs`**
  - Implement `nativeCreate`: Allocates `Box::new(NirvanaEngine::new(width, height))` and returns `Box::into_raw(...) as jlong`.
  - Implement `nativeDestroy`: Safely converts `jlong` back to `Box<NirvanaEngine>` and drops it if pointer != 0.
  - Implement `nativeLoadHtml`, `nativeResize`, `nativeRender`: Retrieve `&mut NirvanaEngine` safely using `(engine_ptr as *mut NirvanaEngine).as_mut()`.
  - Wrap every JNI function body inside `std::panic::catch_unwind` to prevent unwinding across foreign ABI boundaries.
  - Implement `nativeGetPixels`: Creates a `jbyteArray` and copies `engine.pixels()` into Java memory.

- [ ] **Step 4: Verify compilation on host**

Run: `cargo check -p nirvana_android`
Expected: Clean compilation with 0 errors.

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_android/src/jni_api.rs crates/nirvana_android/src/lib.rs crates/nirvana_android/tests/jni_api_test.rs
git commit -m "feat(android): implement JNI C-ABI bindings and safe pointer lifecycle"
```

---

### Task 3: Surface Blitting & Native Window Buffer Copy (`surface.rs`)

**Files:**
- Create: `crates/nirvana_android/src/surface.rs`
- Modify: `crates/nirvana_android/src/lib.rs`
- Test: `crates/nirvana_android/tests/surface_test.rs`

**Interfaces:**
- Consumes: `NirvanaEngine`
- Produces:
  - `pub fn blit_rgba_to_buffer(src_pixels: &[u8], src_width: usize, src_height: usize, dst_buffer: &mut [u8], dst_stride_bytes: usize)`

- [ ] **Step 1: Write failing tests in `crates/nirvana_android/tests/surface_test.rs`**

```rust
use nirvana_android::surface::blit_rgba_to_buffer;

#[test]
fn test_blit_rgba_with_stride_padding() {
    // 2x2 source image (each pixel is 4 bytes RGBA)
    let src_width = 2;
    let src_height = 2;
    let src_pixels = vec![
        255, 0, 0, 255,   0, 255, 0, 255, // Row 0: Red, Green
        0, 0, 255, 255,   255, 255, 0, 255, // Row 1: Blue, Yellow
    ];

    // Destination buffer has stride of 4 pixels (16 bytes) instead of 2 pixels (8 bytes)
    let dst_stride_bytes = 16;
    let mut dst_buffer = vec![0u8; dst_stride_bytes * src_height];

    blit_rgba_to_buffer(&src_pixels, src_width, src_height, &mut dst_buffer, dst_stride_bytes);

    // Verify Row 0
    assert_eq!(&dst_buffer[0..4], &[255, 0, 0, 255]); // Red
    assert_eq!(&dst_buffer[4..8], &[0, 255, 0, 255]); // Green
    assert_eq!(&dst_buffer[8..16], &[0, 0, 0, 0, 0, 0, 0, 0]); // Stride padding untouched

    // Verify Row 1
    assert_eq!(&dst_buffer[16..20], &[0, 0, 255, 255]); // Blue
    assert_eq!(&dst_buffer[20..24], &[255, 255, 0, 255]); // Yellow
    assert_eq!(&dst_buffer[24..32], &[0, 0, 0, 0, 0, 0, 0, 0]); // Stride padding untouched
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_android --test surface_test`
Expected: FAIL

- [ ] **Step 3: Implement `blit_rgba_to_buffer` in `crates/nirvana_android/src/surface.rs`**
  - Implement row-by-row memory copy taking into account `dst_stride_bytes` and `src_row_bytes = src_width * 4`.
  - Guard against buffer overruns if destination buffer size is smaller than required.
  - Implement conditional compilation for Android `ANativeWindow_lock` / `ANativeWindow_unlockAndPost` when compiling for Android.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_android --test surface_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_android/src/surface.rs crates/nirvana_android/tests/surface_test.rs
git commit -m "feat(android): implement surface framebuffer blitting with stride handling"
```

---

### Task 4: Android Kotlin/Java Interface & Build Script (`NirvanaEngine.kt` & `NirvanaView.kt`)

**Files:**
- Create: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaEngine.kt`
- Create: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaView.kt`
- Create: `scripts/build_android.ps1`
- Create: `scripts/build_android.sh`

**Interfaces:**
- Produces:
  - `class NirvanaEngine : AutoCloseable`
  - `class NirvanaView(context: Context, attrs: AttributeSet?) : SurfaceView(context, attrs), SurfaceHolder.Callback`
  - Shell scripts automating `cargo ndk` builds for Android targets (`arm64-v8a`, `armeabi-v7a`, `x86_64`).

- [ ] **Step 1: Create Kotlin wrapper `NirvanaEngine.kt`**
  - Declare `external fun nativeCreate(width: Int, height: Int): Long`.
  - Declare `external fun nativeDestroy(ptr: Long)`.
  - Declare `external fun nativeLoadHtml(ptr: Long, html: String, css: String)`.
  - Declare `external fun nativeResize(ptr: Long, width: Int, height: Int)`.
  - Declare `external fun nativeRender(ptr: Long)`.
  - Declare `external fun nativeGetPixels(ptr: Long): ByteArray`.
  - Add `companion object { init { System.loadLibrary("nirvana_engine") } }`.
  - Implement `close()` calling `nativeDestroy(ptr)`.

- [ ] **Step 2: Create custom Android View `NirvanaView.kt`**
  - Inherit from `SurfaceView` and implement `SurfaceHolder.Callback`.
  - In `surfaceChanged`, trigger `engine.resize(width, height)` and re-render.
  - Blit pixel buffer onto the Android Canvas/Surface via `holder.lockHardwareCanvas()` or software canvas buffer.
  - In `surfaceDestroyed`, pause rendering safely.

- [ ] **Step 3: Create build scripts (`build_android.ps1`, `build_android.sh`)**
  - Script executing `cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs build --release -p nirvana_android`.

- [ ] **Step 4: Commit**

```bash
git add android scripts
git commit -m "feat(android): provide Kotlin wrapper, NirvanaView SurfaceView, and NDK build scripts"
```

---

### Task 5: End-to-End Integration Test & Full Workspace Verification

**Files:**
- Create: `crates/nirvana_android/tests/integration_test.rs`
- Modify: `progress.md`

- [ ] **Step 1: Write end-to-end integration test `crates/nirvana_android/tests/integration_test.rs`**
  - Test full HTML document with inline CSS styling, layout calculation, rendering, and pixel inspection through `NirvanaEngine`.

- [ ] **Step 2: Run test suite across entire workspace**

Run: `cargo test --workspace`
Expected: ALL PASS across all 9 crates (`nirvana_core`, `nirvana_url`, `nirvana_dom`, `nirvana_html`, `nirvana_css`, `nirvana_style`, `nirvana_layout`, `nirvana_gfx`, `nirvana_android`).

- [ ] **Step 3: Run Clippy linter across entire workspace**

Run: `cargo clippy --workspace --all-targets`
Expected: 0 warnings.

- [ ] **Step 4: Commit and update progress ledger**

```bash
git add progress.md crates/nirvana_android/tests/integration_test.rs
git commit -m "test(android): add end-to-end integration test and verify full workspace health"
```
