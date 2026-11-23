# Design Document: Nirvana Browser Engine

**Date:** 2026-11-23  
**Status:** Ready for Approval  
**Authors:** Aryan & Antigravity  
**Target:** Android (Host Browser: Nirvana Browser on Android)  
**Workspace:** `p:\NirvanaBrowserengine`

---

## 1. Intent & The "Why"
The mission of this project is to build an independent, world-class browser engine for the **Nirvana Browser** on **Android** (which currently relies on Mozilla GeckoView). The vision is to replace GeckoView over time with a custom-built, lightweight, memory-safe, and modular Rust-based engine designed for global web standards.

This is a long-term engineering journey (1–3 years) executed incrementally: building, testing, and verifying every subsystem from foundational principles without rushing.

---

## 2. Approach: Rust-Based Modular Android Engine

- **Architecture:** Pure Rust monorepo with clean separation of concerns across independent crates.
- **Why Rust:**
  - Memory safety out of the box (zero use-after-free, double-free, or buffer overflow vulnerabilities).
  - Fearless concurrency for parallel styling, layout, and rendering pipelines.
  - Native cross-compilation to Android architectures (`aarch64-linux-android`, `x86_64`) producing a native dynamic library (`libnirvana_engine.so`) with JNI bindings for the Android app.
- **Why From Scratch:** Core parsers (URL, HTML, CSS), layout algorithms, and paint lists are developed from specs rather than wrapping pre-existing monolithic engines, giving total ownership and architectural clarity.

---

## 3. Goals, Non-Goals & Anti-Goals

### 3.1 Goals
- **G1: Android-First Embedding:** Provide clean JNI bindings (`nirvana_android`) and native surface rendering (`ANativeWindow` / shared buffer) so the Nirvana Android app can host webviews directly.
- **G2: Global Web Standards (English/International First):** Standard typography, font metrics, line-breaking, and CSS layout conforming to WHATWG HTML and W3C CSS specifications.
- **G3: Strict Modular Layering:** Every crate is isolated, zero cyclic dependencies, and testable independently in headless mode.
- **G4: Multi-Tier Verification:**
  - Unit tests for every parser state machine and algorithm.
  - Headless visual regression testing (rendering HTML/CSS to pixel-perfect PNGs).
  - Android APK test harness to verify on-device performance and rendering.
- **G5: Arena-Based Memory Architecture:** Use index-based arena storage (`NodeId(u32)`) for the DOM and Layout trees to ensure zero reference cycles, cache locality, and `Send + Sync` thread safety.

### 3.2 Non-Goals (Initial Milestones)
- Full 100% WHATWG/CSS3 specification coverage on Day 1. The engine focuses first on a foundational subset (box model, block/inline layout, typography, colors, borders, margins, padding, images).
- Immediate custom JavaScript JIT engine (initial phases focus on static document layout and visual rendering; JS will be integrated via an embeddable runtime like QuickJS/Boa in later phases).
- Desktop GUI windowing systems (all engine host code targets Android and headless test runners).

### 3.3 Anti-Goals
- **Monolithic Entanglement:** No coupling between DOM data structures, layout calculation, and graphics backends.
- **Premature Android JNI Coupling in Core:** Core crates (`nirvana_html`, `nirvana_layout`, etc.) must remain pure, platform-agnostic Rust; only `nirvana_android` / `nirvana_engine` handles JNI and Android-specific APIs.
- **Uncontrolled Unsafe Code:** Strictly `#![forbid(unsafe_code)]` in all core parsing and layout crates.

---

## 4. Architecture & System Decomposition

```
nirvana/
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── nirvana_core/           # Primitives: Arena/SlotMap, Geometry (Point, Size, Rect), Colors
│   ├── nirvana_url/            # WHATWG URL parser (http, https, file, data schemes)
│   ├── nirvana_net/            # Network & resource loader (HTTP/HTTPS client & file loader)
│   ├── nirvana_dom/            # DOM Tree (Document, Element, Text, Arena-backed Node IDs)
│   ├── nirvana_html/           # HTML5 streaming tokenizer (state machine) & tree builder
│   ├── nirvana_css/            # CSS3 tokenizer, parser, selectors, and declaration blocks
│   ├── nirvana_style/          # Style cascade, specificity, inheritance & computed values
│   ├── nirvana_layout/         # Layout tree (Box Model, BFC, Inline Formatting Context)
│   ├── nirvana_gfx/            # Display list, painting commands, 2D rasterizer (tiny-skia)
│   ├── nirvana_engine/         # Core coordinator (parses URL -> DOM -> Style -> Layout -> Paint)
│   └── nirvana_android/        # JNI bindings, ANativeWindow renderer, Android View bridge
└── tests/                      # Conformance test suites, HTML/CSS test fixtures, PNG goldens
```

---

## 5. Phased Roadmap

### Phase 1: Workspace Setup & Core Primitives
1. Initialize Cargo workspace with `nirvana_core` and `nirvana_url`.
2. Implement core geometry (`Rect`, `Point`, `Size`), `Color`, and arena allocator abstractions.
3. Build the WHATWG URL parser from scratch with comprehensive unit tests.

### Phase 2: DOM Representation & HTML5 Parser
1. Implement `nirvana_dom`: Arena-backed DOM hierarchy (`Document`, `Element`, `TextNode`, `Attr`).
2. Implement `nirvana_html`:
   - Streaming input pre-processing.
   - HTML5 Tokenizer state machine (tags, attributes, characters, comments, doctype).
   - Tree construction algorithm creating proper DOM nodes.

### Phase 3: CSS Parser & Style Engine
1. Implement `nirvana_css`: CSS tokenization, rule parsing, declaration lists, and selectors (tag, class, ID).
2. Implement `nirvana_style`: Style cascade, specificity sorting, user-agent base styles, inheritance, and computed style tree.

### Phase 4: Layout Engine (The Box Model)
1. Implement `nirvana_layout`:
   - Generation of layout boxes (Block boxes, Inline boxes, Anonymous boxes) from the styled DOM.
   - Margin, border, padding, and content area geometry calculation.
   - Block formatting context and basic inline text flow.

### Phase 5: 2D Painting & Rasterization
1. Implement `nirvana_gfx`:
   - Display list generation (draw background, draw border, draw text run).
   - High-performance software rasterizer with `tiny-skia` outputting raw RGBA pixel buffers.
   - Headless image test harness producing PNG snapshots.

### Phase 6: Android Integration & JNI Shell
1. Implement `nirvana_android`:
   - Cargo NDK build scripts producing `libnirvana_engine.so`.
   - JNI interface: `createEngine()`, `loadUrl(String)`, `renderToSurface(Surface)`.
   - Android sample app embedding Nirvana Engine to display live rendered web content.
