# Phase 14: Engine Hardening, Performance Profiling, Memory Audit & Final Android Production Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the final hardening, resilience limits, performance telemetry, memory audit, Android documentation, and production readiness of the sovereign Nirvana Browser Engine.

**Architecture:** Fortify the engine against malformed input, recursion overflow, decompression bombs, and memory leaks. Implement sub-16ms frame timing telemetry across style, layout, and rasterization passes. Provide complete architecture documentation and verify 100% test coverage and 0 clippy warnings under strict `#![forbid(unsafe_code)]`.

**Tech Stack:** Pure Rust (11 core crates with `#![forbid(unsafe_code)]`), `ureq` + `rustls`, `boa_engine` 0.22, JNI FFI bindings, Android Kotlin integration.

**Spec:** Sovereign Browser Engine Architecture for Nirvana Browser on Android.

## Global Constraints

- Strict `#![forbid(unsafe_code)]` enforced across all 11 core Rust crates.
- In `nirvana_android`, unsafe code is strictly confined to the JNI FFI boundary, protected by `std::panic::catch_unwind` and pointer validation.
- Zero C/C++ compiler (`cl.exe`) dependencies on Windows host.
- Pure Rust TLS using `rustls` through `ureq`.
- 60 FPS animation budget / sub-16ms target for typical documents.
- 0 warnings on `cargo clippy --workspace --all-targets`.

## Review Focus

1. Deep recursion in HTML parsing (e.g. 1000+ nested tags) does not exhaust the stack or panic.
2. Malformed or huge images do not cause OOM crashes or decompression allocation panics.
3. Rapid engine creation, rendering, and destruction cycles in JNI release all native resources cleanly without memory leaks.
4. Frame metrics telemetry accurately measures phase durations without incurring measurement overhead.
5. All 12 crates in the workspace compile cleanly and pass all unit, integration, and E2E test suites.

---

### Task 1: Engine Hardening, Malformed Input Recovery & Recursion Depth Guards

**Files:**
- Modify: `crates/nirvana_html/src/parser.rs`
- Modify: `crates/nirvana_gfx/src/image.rs`
- Modify: `crates/nirvana_android/src/engine.rs`
- Test: `crates/nirvana_android/tests/hardening_resilience_test.rs`

- [ ] **Step 1.1: Write failing tests for engine resilience and bounds**
  Create `crates/nirvana_android/tests/hardening_resilience_test.rs`:
  - 1000+ deeply nested `<div>` tags parsed without stack overflow.
  - Zero-width / zero-height viewport resizing handled gracefully.
  - Unclosed tags, malformed CSS, and invalid data URIs render gracefully without crashing.
  - Huge image dimensions bounded by maximum size checks.

- [ ] **Step 1.2: Run test to confirm failure**
  Run `cargo test -p nirvana_android --test hardening_resilience_test`.

- [ ] **Step 1.3: Implement depth limits and dimension validation**
  - In `crates/nirvana_html/src/parser.rs`: add max tree depth limit (e.g. 512 levels) to avoid recursion stack exhaustion.
  - In `crates/nirvana_gfx/src/image.rs`: add maximum dimension guard (e.g. max 8192x8192 or 64MB uncompressed buffer) to prevent decompression bombs.
  - In `crates/nirvana_android/src/engine.rs`: ensure viewport dimensions `width` and `height` are clamped to safe minimums (`.max(1)`) and maximums (`.min(16384)`).

- [ ] **Step 1.4: Run tests to verify pass**
  Run `cargo test -p nirvana_android --test hardening_resilience_test`.

- [ ] **Step 1.5: Commit Task 1**
  `git add crates/ ; git commit -m "feat(engine): add recursion depth guards, image dimension bounds, and malformed input hardening"`

---

### Task 2: Frame Timing Telemetry & Performance Profiling Harness

**Files:**
- Modify: `crates/nirvana_android/src/engine.rs`
- Modify: `crates/nirvana_android/src/jni_api.rs`
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaEngine.kt`
- Test: `crates/nirvana_android/tests/performance_bench_test.rs`

- [ ] **Step 2.1: Write failing tests for performance telemetry**
  Create `crates/nirvana_android/tests/performance_bench_test.rs`:
  - `engine.render()` records phase durations: style computation, layout calculation, and rasterization.
  - `engine.frame_metrics()` exposes `parse_time_ms`, `style_time_ms`, `layout_time_ms`, `raster_time_ms`, and `total_frame_time_ms`.
  - Frame metrics are updated accurately across successive renders.
  - Sub-16ms latency verified for standard 400x600 documents.

- [ ] **Step 2.2: Run test to confirm failure**
  Run `cargo test -p nirvana_android --test performance_bench_test`.

- [ ] **Step 2.3: Implement `FrameMetrics` and telemetry collection**
  - In `crates/nirvana_android/src/engine.rs`:
    - Define `#[derive(Debug, Clone, Copy, Default)] pub struct FrameMetrics`.
    - Record `std::time::Instant` milestones during `load_html`, `rebuild_stylesheets`, `layout`, and `render`.
    - Expose `pub fn frame_metrics(&self) -> FrameMetrics`.
  - In `crates/nirvana_android/src/jni_api.rs`:
    - Add `Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetFrameMetrics` returning a `jfloatArray` of `[style_ms, layout_ms, raster_ms, total_ms]`.
  - In `NirvanaEngine.kt`:
    - Expose `fun getFrameMetrics(): FrameMetrics`.

- [ ] **Step 2.4: Run tests to verify pass**
  Run `cargo test -p nirvana_android --test performance_bench_test`.

- [ ] **Step 2.5: Commit Task 2**
  `git add crates/ android/ ; git commit -m "feat(telemetry): add high-precision frame timing metrics and performance telemetry"`

---

### Task 3: Memory Audit, Lifecycle Stress Testing & Resource Clean-up

**Files:**
- Modify: `crates/nirvana_android/src/engine.rs`
- Test: `crates/nirvana_android/tests/memory_lifecycle_test.rs`

- [ ] **Step 3.1: Write failing tests for memory lifecycle and rapid recycling**
  Create `crates/nirvana_android/tests/memory_lifecycle_test.rs`:
  - Rapid allocation and destruction of 50 engine instances with complex DOMs, images, and scripts.
  - Document navigation frees prior document DOM, style tree, display list, and layout box trees cleanly.
  - Cache clearing APIs: `engine.clear_image_cache()`, `engine.clear_cookies()`, `engine.clear_storage()`.

- [ ] **Step 3.2: Run test to confirm failure**
  Run `cargo test -p nirvana_android --test memory_lifecycle_test`.

- [ ] **Step 3.3: Implement resource pruning and cache management**
  - In `crates/nirvana_android/src/engine.rs`:
    - Add `pub fn clear_image_cache(&mut self)` to clear decoded image bitmaps.
    - Explicitly drop old layout box trees and style trees before allocating fresh documents.
    - Add `pub fn shrink_to_fit(&mut self)` to release capacity on canvas framebuffer and caches.

- [ ] **Step 3.4: Run tests to verify pass**
  Run `cargo test -p nirvana_android --test memory_lifecycle_test`.

- [ ] **Step 3.5: Commit Task 3**
  `git add crates/ ; git commit -m "feat(engine): implement cache pruning, memory cleanup, and lifecycle stress hardening"`

---

### Task 4: Complete Production Documentation & Architecture Manual

**Files:**
- Create/Modify: `README.md`
- Create: `docs/architecture.md`
- Create: `docs/android_integration.md`

- [ ] **Step 4.1: Write complete project README.md**
  - Engine overview, features, 12 sovereign crates topology, building for host and Android, and testing instructions.
- [ ] **Step 4.2: Write docs/architecture.md**
  - In-depth pipeline documentation: HTML Tokenization/Parsing -> CSS Cascading/Inheritance -> Layout Geometry -> DisplayList/2D Rasterization -> Font Glyphs -> Resource Loading & TLS -> JavaScript & DOM Event Loop -> Web Security & Storage.
- [ ] **Step 4.3: Write docs/android_integration.md**
  - Guide for Android developers: embedding `NirvanaView`, configuring touch events, soft keyboard IME, cookie lifecycle, and performance profiling.
- [ ] **Step 4.4: Commit Task 4**
  `git add README.md docs/ ; git commit -m "docs: add comprehensive engine architecture, API reference, and Android integration manual"`

---

### Task 5: End-to-End Final Production Verification & Workspace Cleanliness

**Files:**
- Modify: `progress.md`

- [ ] **Step 5.1: Run full workspace test suite**
  Run `cargo test --workspace` across all 12 crates. All tests must pass.
- [ ] **Step 5.2: Run workspace linter**
  Run `cargo clippy --workspace --all-targets` (must be 0 warnings).
- [ ] **Step 5.3: Verify `#![forbid(unsafe_code)]` compliance**
  Ensure all 11 core crates enforce `#![forbid(unsafe_code)]`.
- [ ] **Step 5.4: Independent Code Review & Final Sign-Off**
  - Generate diff for Phase 14.
  - Invoke Senior Browser Systems & Release Reviewer subagent.
  - Address any findings until `[APPROVAL]` is obtained.
  - Update `progress.md` to declare all 14 phases complete.
- [ ] **Step 5.5: Final Commit**
  `git add progress.md ; git commit -m "docs: mark Phase 14 and Nirvana Browser Engine as fully completed and signed off"`
