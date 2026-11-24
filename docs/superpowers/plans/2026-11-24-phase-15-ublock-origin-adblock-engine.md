# Phase 15: Sovereign uBlock Origin Ad-Block & Content Filter Engine Implementation Plan

## 1. Overview & Purpose
Phase 15 introduces `nirvana_adblock`, a pure-Rust, high-performance content filtering engine modeled after uBlock Origin (uBO) and AdBlock Plus (ABP) syntax. It empowers Nirvana Browser with wide-spectrum blocking capabilities:
- **Network Request Filtering:** Intercepting and terminating advertising and tracking requests before network transmission in `nirvana_net`.
- **Cosmetic Filtering:** Extracting CSS element hiding rules and injecting them into the document style cascade in `nirvana_style` to eliminate empty ad banners.
- **Scriptlet Defusers:** Neutralizing anti-adblock detection and analytics stubs (`window.ga`, `window.google_tag_manager`) before page scripts execute.
- **Telemetry & Live Shield:** Real-time blocked request counters exposed to the JNI and Kotlin Android UI.

---

## 2. Architectural Blueprint & Crate Structure

```
crates/nirvana_adblock/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Core exports and #![forbid(unsafe_code)]
    ├── filter.rs               # NetworkFilter, CosmeticFilter, FilterAction, ContentType
    ├── parser.rs               # ABP/uBlock rule syntax parser
    ├── matcher.rs              # Fast domain/URL rule matcher using hash sets & tries
    ├── cosmetic.rs             # Domain-specific CSS generator (display: none !important)
    ├── scriptlets.rs           # Neutralizing stubs (Google Analytics, GTM, trackers)
    ├── engine.rs               # Unified AdBlockEngine coordinating network & cosmetic passes
    └── rulesets/               # Compiled built-in sovereign ruleset (EasyList + EasyPrivacy subset)
```

---

## 3. Tasks Breakdown

### Task 1: Scaffolding `nirvana_adblock` & Filter Model
- Create `crates/nirvana_adblock` with `#![forbid(unsafe_code)]`.
- Add to workspace `Cargo.toml`.
- Define data structures:
  - `ContentType`: `Script`, `Image`, `Stylesheet`, `Subdocument`, `XmlHttpRequest`, `Other`.
  - `FilterAction`: `Block`, `Allow` (whitelist), `Redirect`.
  - `NetworkRule`: pattern, domain anchors, content type bitflags, third-party flag, action.
  - `CosmeticRule`: domains (include/exclude), selector.

### Task 2: Filter List Parser (ABP / uBO Syntax)
- Support rule syntax:
  - Domain anchor: `||example.com^`
  - URL prefixes/suffixes: `|http://...`, `.../banner.js|`
  - Wildcards: `*/ads/*`
  - Separator: `^` (matches `/`, `?`, `:`, etc.)
  - Modifiers: `$script`, `$image`, `$stylesheet`, `$third-party`, `$domain=a.com|~b.com`
  - Whitelist: `@@||safe.com^`
  - Cosmetic filters: `##.ad-banner`, `###ads`, `example.com##.sponsor`
  - Comments: `! ...`

### Task 3: Fast Pattern Matcher & Domain Index
- Fast lookup indexing:
  - Direct domain hash table for exact `||domain^` matches.
  - Suffix tree / regex / substring matcher for wildcard rules.
  - First-party vs Third-party origin comparison based on registrable domain.

### Task 4: Cosmetic Filtering & Scriptlet Defusers
- `CosmeticEngine`: Matches document origin against active cosmetic rules and returns compiled minified CSS (e.g. `selector { display: none !important; }`).
- `ScriptletEngine`: Generates defuser scripts injected at `document_start` to prevent site breakage when tracking scripts are blocked.

### Task 5: Engine Integration, JNI & Kotlin Bindings
- Wire `AdBlockEngine` into `ResourceLoader` in `nirvana_net`:
  - Outgoing HTTP/HTTPS subresource requests checked against adblock rules.
  - If blocked: request cancelled immediately (status 0 / BlockedByExtension).
  - Increment blocked counter.
- Wire cosmetic CSS into `NirvanaEngine::load_html` and `NirvanaEngine::navigate`.
- JNI Exports in `nirvana_android::jni_api`:
  - `nativeGetBlockedCount(enginePtr: Long): Long`
  - `nativeSetAdBlockEnabled(enginePtr: Long, enabled: Boolean)`
  - `nativeAddAdBlockRule(enginePtr: Long, rule: String)`
- Kotlin methods in `NirvanaEngine.kt`:
  - `fun getBlockedRequestsCount(): Long`
  - `fun setAdBlockEnabled(enabled: Boolean)`
  - `fun addCustomFilterRule(rule: String)`

### Task 6: Unit, Integration & E2E Tests
- `adblock_parser_test.rs`: Parse network rules, modifiers, cosmetic rules, comments.
- `adblock_matcher_test.rs`: Match URLs with domains, content types, third-party flags.
- `adblock_cosmetic_test.rs`: Verify CSS generation for matching domains.
- `engine_adblock_e2e_test.rs`: Verify that pages with ad scripts have ad requests blocked, blocked counter increments, and ad elements are visually hidden.

### Task 7: Whole-Branch Verification & Independent Code Review
- Full workspace test pass (`cargo test --workspace`).
- Clippy validation (`cargo clippy --workspace --all-targets`).
- Independent code review by Senior Browser Security & Systems Reviewer subagent.
