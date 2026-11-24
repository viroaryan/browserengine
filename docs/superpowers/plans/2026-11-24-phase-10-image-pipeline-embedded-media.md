# Phase 10: Image Pipeline & Embedded Media (`<img>` Tag Support)

**Status:** IN PROGRESS  
**Date:** 2026-11-24  
**Workspace:** `P:\NirvanaBrowserengine`  
**Goal:** Deliver a sovereign, high-performance, pure-Rust image decoding, intrinsic layout sizing, and 2D bitmap rasterization pipeline for the Nirvana Browser Engine.

---

## 1. Overview & Architectural Motivation

Images constitute the majority of visual content on modern websites. Until now, the Nirvana Browser Engine has rendered typography, vector geometries, borders, and backgrounds, but elements like `<img src="...">` lacked intrinsic dimensions, image decoding, and pixel rasterization in the display list.

Phase 10 introduces:
1. **Pure-Rust Image Decoding (`nirvana_gfx::image`)**: Zero C-dependencies decoding PNG and JPEG formats from raw network/data-URI bytes into 32-bit RGBA buffers.
2. **Replaced Element Sizing & Intrinsic Aspect Ratio (`nirvana_layout`)**: Respecting intrinsic dimensions, CSS `width`/`height` overrides, aspect ratio preservation for `auto` dimensions, and HTML `width`/`height` attributes.
3. **Display List Bitmap Rasterization (`nirvana_gfx::canvas`)**: New `DisplayItem::DrawImage` primitive rendered onto `tiny_skia` canvas with bilinear scaling and scroll offset support.
4. **Engine Image Asset Orchestration (`nirvana_android::engine`)**: DOM traversal discovering `<img>` elements, resolving relative URLs via `nirvana_url::Url::join`, fetching assets via `nirvana_net::ResourceLoader`, and caching decoded images.
5. **Resilient Broken-Image Fallbacks**: Graceful handling of 404s, corrupt payloads, and rendering `alt` text.

---

## 2. Detailed Task Breakdown

### Task 1: Image Representation & Pure-Rust Image Decoder (`nirvana_gfx`)
- Add `image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }` to `crates/nirvana_gfx/Cargo.toml`.
- Create `crates/nirvana_gfx/src/image.rs`:
  - `DecodedImage`: stores `width: u32`, `height: u32`, and `rgba_pixels: Vec<u8>` (premultiplied or straight RGBA8).
  - `decode_image(bytes: &[u8]) -> Result<DecodedImage, ImageError>`.
  - Type alias `ImageMap = HashMap<String, Arc<DecodedImage>>`.
- Write unit tests in `crates/nirvana_gfx/tests/image_decode_test.rs` validating PNG and JPEG byte decoding.

### Task 2: Replaced Element Box Model & Intrinsic Sizing (`nirvana_layout`)
- In `crates/nirvana_layout/src/box_model.rs` and `block.rs`:
  - Support replaced element layout for `tag_name == "img"`.
  - Intrinsic aspect ratio logic:
    - If CSS `width` and `height` are both fixed lengths, use them.
    - If CSS `width` is set and `height` is `auto`, `height = width * (intrinsic_height / intrinsic_width)`.
    - If CSS `height` is set and `width` is `auto`, `width = height * (intrinsic_width / intrinsic_height)`.
    - If neither CSS dimension is set, fall back to HTML `width` / `height` attributes if present.
    - If no attributes, use intrinsic image width/height (or 16x16 fallback if image missing).
- Write unit tests in `crates/nirvana_layout/tests/image_layout_test.rs`.

### Task 3: Display List Image Item & tiny-skia Bitmap Rasterization (`nirvana_gfx`)
- In `crates/nirvana_gfx/src/display_list.rs`:
  - Add `DisplayItem::DrawImage { rect: Rect, image: Arc<DecodedImage>, alt: Option<String> }`.
  - Emit `DrawImage` when building display list for layout boxes with associated image assets.
- In `crates/nirvana_gfx/src/canvas.rs`:
  - Handle `DisplayItem::DrawImage`:
    - Scale decoded image pixmap into target bounds `rect` offset by `(-scroll_x, -scroll_y)`.
    - Draw onto canvas using `tiny_skia::PixmapRef` and `draw_pixmap` with bilinear sampling.
- Write unit tests in `crates/nirvana_gfx/tests/image_paint_test.rs`.

### Task 4: DOM Image Discovery & Asynchronous Resource Fetching (`nirvana_android`)
- In `crates/nirvana_android/src/engine.rs`:
  - `discover_images(dom: &DomTree) -> Vec<(NodeId, String)>`: collects all `<img src="...">` tags.
  - In `NirvanaEngine::navigate` and `load_html`:
    - Resolve relative image URLs against document base URL (`url.join(&src)`).
    - Fetch image resources via `self.loader.fetch(&img_url)`.
    - Decode bytes into `Arc<DecodedImage>` and populate `image_map: Arc<HashMap<String, Arc<DecodedImage>>>`.
    - Pass `image_map` to layout and painting pipeline.
- Write unit tests in `crates/nirvana_android/tests/image_pipeline_test.rs`.

### Task 5: Broken Image Fallback & Alt Text Support
- In `nirvana_gfx`:
  - When image fails to load or decode, render fallback placeholder rect (outline + light gray fill).
  - If `alt` attribute is present, render alt text inside the placeholder using `nirvana_font`.
- Write unit tests in `crates/nirvana_gfx/tests/image_fallback_test.rs`.

### Task 6: End-to-End Image Pipeline Test & Full Workspace Verification
- In `crates/nirvana_android/tests/image_e2e_test.rs`:
  - Load HTML page with base64 PNG image, local file JPEG, and remote URL.
  - Verify layout coordinates, pixel colors at exact image locations, and scroll behavior.
- Run `cargo test --workspace` across all crates (100% passing).
- Run `cargo clippy --workspace --all-targets` (0 warnings).
- Strict `#![forbid(unsafe_code)]` compliance.
- Independent code reviewer sign-off.
