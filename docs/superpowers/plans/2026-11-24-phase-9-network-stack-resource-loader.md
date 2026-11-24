# Phase 9: Network Stack & Resource Loader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `nirvana_net` crate and resource loading pipeline enabling `NirvanaEngine` to navigate live URLs (`http://`, `https://`, `file://`, `data:`), fetch and parse external stylesheets (`<link rel="stylesheet">`), and support asynchronous web page loading on Android.

**Architecture:** Create `nirvana_net` with modular protocol handlers (`HttpHandler` via `ureq` with `rustls`, `FileHandler`, `DataUriHandler`) and an in-memory resource cache. Extend `nirvana_url::Url` with WHATWG URL resolution (`Url::join`) and string serialization. Integrate `ResourceLoader` into `NirvanaEngine::navigate`, discovering and fetching external stylesheets before rendering, and expose `loadUrl` with background worker threading in Android `NirvanaView.kt`.

**Tech Stack:** Rust 2024 edition, `ureq` (TLS with pure Rust `rustls` + `webpki-roots`, zero OpenSSL/C dependencies), `nirvana_url`, JNI, Android Kotlin background threading.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints

- Pure Rust with strict memory safety: `#![forbid(unsafe_code)]` at crate roots of all core crates including `nirvana_net`.
- Zero C-library TLS dependencies: Use pure Rust TLS (`rustls` with embedded roots) to guarantee seamless Android NDK cross-compilation across `aarch64` and `x86_64`.
- JNI boundary safety: all exported C-ABI functions must wrap execution in `std::panic::catch_unwind`.
- Zero clippy warnings across `--workspace --all-targets`.
- Relative URL resolution must conform to WHATWG path and base resolution rules.

## Review Focus

1. **Relative URL Resolution Edge Cases**: Resolving `../`, root-relative `/path`, scheme-relative `//host/path`, and query parameters against a base document URL must never panic or produce malformed URLs.
2. **Data URI Decoding Robustness**: Correctly handling both plaintext URL-encoded payloads and base64 payloads (e.g. `data:text/css;base64,...`) without memory corruption or panics on malformed input.
3. **External Stylesheet Pipeline Synchronization**: Fetching multiple `<link rel="stylesheet">` sheets must accumulate in author sheet order and cascade properly against inline styles before layout rasterization.
4. **Network Error Handling & Graceful Degradation**: Failed stylesheet fetches (404, DNS error, timeout) must not abort document rendering; the engine must log the failure and render remaining document styles.
5. **Android Thread Concurrency Safety**: Network I/O must never block Android's main UI thread; `NirvanaView.loadUrl` must execute off-thread and blit the resulting framebuffer on the UI thread via `SurfaceHolder`.

---

### Task 1: WHATWG URL Resolution & String Serialization (`nirvana_url`)

**Files:**
- Modify: `crates/nirvana_url/src/parser.rs`
- Modify: `crates/nirvana_url/src/lib.rs`
- Test: `crates/nirvana_url/tests/url_join_test.rs`

**Interfaces:**
- Produces in `nirvana_url::Url`:
  - `pub fn as_str(&self) -> String`
  - `pub fn join(&self, input: &str) -> Result<Url, UrlError>`
  - `impl std::fmt::Display for Url`

- [ ] **Step 1: Write failing tests in `crates/nirvana_url/tests/url_join_test.rs`**

```rust
use nirvana_url::Url;

#[test]
fn test_url_display_and_as_str() {
    let url = Url::parse("https://example.com:8080/path/doc.html?query=1#frag").unwrap();
    assert_eq!(url.as_str(), "https://example.com:8080/path/doc.html?query=1#frag");
    assert_eq!(format!("{}", url), "https://example.com:8080/path/doc.html?query=1#frag");
}

#[test]
fn test_url_join_relative_paths() {
    let base = Url::parse("https://example.com/blog/article.html").unwrap();

    // Sibling relative path
    let sibling = base.join("style.css").unwrap();
    assert_eq!(sibling.as_str(), "https://example.com/blog/style.css");

    // Parent relative path
    let parent = base.join("../assets/main.css").unwrap();
    assert_eq!(parent.as_str(), "https://example.com/assets/main.css");

    // Root-relative path
    let root = base.join("/app.css").unwrap();
    assert_eq!(root.as_str(), "https://example.com/app.css");

    // Scheme-relative path
    let scheme_rel = base.join("//cdn.example.org/lib.js").unwrap();
    assert_eq!(scheme_rel.as_str(), "https://cdn.example.org/lib.js");

    // Absolute URL override
    let abs = base.join("http://other.com/index.html").unwrap();
    assert_eq!(abs.as_str(), "http://other.com/index.html");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nirvana_url --test url_join_test`
Expected: FAIL (missing `join`, `as_str`, `Display`)

- [ ] **Step 3: Implement `as_str`, `Display`, and `join` in `crates/nirvana_url/src/parser.rs`**
  - Implement `as_str(&self) -> String`:
    Formats scheme, host, custom port, path, query, and fragment into canonical URL string.
  - Implement `Display for Url`: delegates to `as_str()`.
  - Implement `join(&self, input: &str) -> Result<Url, UrlError>`:
    1. If `input` parses as a standalone URL with a scheme (and not a single drive letter), return `Url::parse(input)`.
    2. If `input.starts_with("//")`, copy `self.scheme` and parse authority + path.
    3. If `input.starts_with('/')`, replace path with `input` (preserving host, port, scheme).
    4. Otherwise, compute parent directory of `self.path` (up to last `/`) and append `input`, then run `normalize_path`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p nirvana_url --test url_join_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_url
git commit -m "feat(url): implement URL relative resolution, canonical serialization, and Display"
```

---

### Task 2: `nirvana_net` Core Abstractions, Error, File & Data URI Handlers

**Files:**
- Create: `crates/nirvana_net/Cargo.toml`
- Create: `crates/nirvana_net/src/lib.rs`
- Create: `crates/nirvana_net/src/error.rs`
- Create: `crates/nirvana_net/src/response.rs`
- Create: `crates/nirvana_net/src/handler.rs`
- Modify: `Cargo.toml` (add `crates/nirvana_net` to workspace members)
- Test: `crates/nirvana_net/tests/file_data_handler_test.rs`

**Interfaces:**
- Produces:
  - `pub enum NetError`
  - `pub struct Response { pub status: u16, pub headers: HashMap<String, String>, pub body: Vec<u8>, pub mime_type: Option<String> }`
  - `Response::text(&self) -> Result<&str, std::str::Utf8Error>`
  - `pub trait ProtocolHandler: Send + Sync`
  - `pub struct FileHandler`
  - `pub struct DataUriHandler`

- [ ] **Step 1: Write failing tests in `crates/nirvana_net/tests/file_data_handler_test.rs`**

```rust
use nirvana_net::handler::{DataUriHandler, FileHandler, ProtocolHandler};
use nirvana_url::Url;

#[test]
fn test_data_uri_handler_plain_and_base64() {
    let handler = DataUriHandler;
    assert!(handler.can_handle("data"));

    // Plain text data URI
    let plain_url = Url::parse("data:text/html,<h1>Hello</h1>").unwrap();
    let resp = handler.fetch(&plain_url).unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.mime_type.as_deref(), Some("text/html"));
    assert_eq!(resp.text().unwrap(), "<h1>Hello</h1>");

    // Base64 encoded data URI ("body { color: red; }")
    let b64_url = Url::parse("data:text/css;base64,Ym9keSB7IGNvbG9yOiByZWQ7IH0=").unwrap();
    let resp_b64 = handler.fetch(&b64_url).unwrap();
    assert_eq!(resp_b64.status, 200);
    assert_eq!(resp_b64.mime_type.as_deref(), Some("text/css"));
    assert_eq!(resp_b64.text().unwrap(), "body { color: red; }");
}

#[test]
fn test_file_handler_reads_local_file() {
    let handler = FileHandler;
    assert!(handler.can_handle("file"));

    let cargo_path = std::env::current_dir().unwrap().join("Cargo.toml");
    let file_url_str = format!("file:///{}", cargo_path.to_str().unwrap().replace('\\', "/"));
    let url = Url::parse(&file_url_str).unwrap();

    let resp = handler.fetch(&url).unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.text().unwrap().contains("[workspace]"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p nirvana_net --test file_data_handler_test`
Expected: FAIL (crate / structs don't exist yet)

- [ ] **Step 3: Implement crate, `Response`, `NetError`, `FileHandler`, and `DataUriHandler`**
  - Scaffold `crates/nirvana_net/Cargo.toml` with `nirvana_core` and `nirvana_url`.
  - Add `crates/nirvana_net` to workspace `Cargo.toml`.
  - In `crates/nirvana_net/src/error.rs`: define `NetError`.
  - In `crates/nirvana_net/src/response.rs`: implement `Response` with `status`, `headers`, `body`, `mime_type`, `text()`.
  - In `crates/nirvana_net/src/handler.rs`:
    - `FileHandler`: reads from `url.path()`, inspects file extension to guess MIME (`html`, `css`, `txt`, `png`, `ttf`), returns bytes.
    - `DataUriHandler`: parses `data:[mediatype][;base64],<data>`, handles URL percentage unescaping and base64 decoding (using a simple zero-dependency base64 decoder).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p nirvana_net --test file_data_handler_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_net Cargo.toml Cargo.lock
git commit -m "feat(net): implement core network response, file handler, and data URI decoder"
```

---

### Task 3: HTTP/HTTPS Handler & Resource Loader with In-Memory Cache

**Files:**
- Modify: `crates/nirvana_net/Cargo.toml`
- Create: `crates/nirvana_net/src/http.rs`
- Create: `crates/nirvana_net/src/cache.rs`
- Create: `crates/nirvana_net/src/loader.rs`
- Modify: `crates/nirvana_net/src/lib.rs`
- Test: `crates/nirvana_net/tests/loader_test.rs`

**Interfaces:**
- Produces in `nirvana_net`:
  - `pub struct HttpHandler`
  - `pub struct ResourceCache`
  - `pub struct ResourceLoader`:
    - `ResourceLoader::new() -> Self`
    - `ResourceLoader::fetch(&self, url: &Url) -> Result<Response, NetError>`
    - `ResourceLoader::fetch_string(&self, url_str: &str) -> Result<Response, NetError>`

- [ ] **Step 1: Write failing tests in `crates/nirvana_net/tests/loader_test.rs`**

```rust
use nirvana_net::loader::ResourceLoader;
use nirvana_url::Url;

#[test]
fn test_resource_loader_dispatches_data_and_file() {
    let loader = ResourceLoader::new();

    // Data URI dispatch
    let resp = loader.fetch_string("data:text/plain,Nirvana").unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.text().unwrap(), "Nirvana");

    // Cache hit verification
    let resp2 = loader.fetch_string("data:text/plain,Nirvana").unwrap();
    assert_eq!(resp2.text().unwrap(), "Nirvana");
}

#[test]
fn test_resource_loader_unsupported_scheme() {
    let loader = ResourceLoader::new();
    let err = loader.fetch_string("ftp://example.com/file").unwrap_err();
    assert!(err.to_string().contains("Unsupported scheme"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_net --test loader_test`
Expected: FAIL

- [ ] **Step 3: Implement `HttpHandler`, `ResourceCache`, and `ResourceLoader`**
  - Add `ureq = { version = "3.0", default-features = true }` to `crates/nirvana_net/Cargo.toml`.
  - In `crates/nirvana_net/src/http.rs`:
    Implement `HttpHandler` using `ureq` with standard User-Agent header, redirect following, 10s timeout, and status code checks.
  - In `crates/nirvana_net/src/cache.rs`:
    Implement `ResourceCache` using thread-safe `RwLock<HashMap<String, Response>>`.
  - In `crates/nirvana_net/src/loader.rs`:
    Implement `ResourceLoader` bundling `HttpHandler`, `FileHandler`, `DataUriHandler`, and `ResourceCache`.
    Check cache first; on miss, dispatch to appropriate protocol handler, store in cache on success.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_net --test loader_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_net
git commit -m "feat(net): implement HTTP/HTTPS handler, in-memory cache, and resource loader"
```

---

### Task 4: Engine Document Navigation & External Stylesheet Discovery (`nirvana_android`)

**Files:**
- Modify: `crates/nirvana_android/Cargo.toml`
- Modify: `crates/nirvana_android/src/engine.rs`
- Modify: `crates/nirvana_android/src/jni_api.rs`
- Modify: `crates/nirvana_android/src/lib.rs`
- Test: `crates/nirvana_android/tests/navigate_test.rs`

**Interfaces:**
- Produces in `NirvanaEngine`:
  - `pub fn navigate(&mut self, url_str: &str) -> Result<(), String>`
  - `pub fn current_url(&self) -> Option<&str>`
- Produces in JNI:
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeNavigate(env, class, ptr, url: JString) -> jboolean`
  - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetCurrentUrl(env, class, ptr) -> jstring`

- [ ] **Step 1: Write failing tests in `crates/nirvana_android/tests/navigate_test.rs`**

```rust
use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_navigate_data_uri_with_linked_stylesheet() {
    let mut engine = NirvanaEngine::new(400, 300);

    // Document contains a linked external stylesheet via data: URI
    let html = r#"
        <html>
            <head>
                <link rel="stylesheet" href="data:text/css,body { background-color: #00ff00; }">
            </head>
            <body>
                <div>Green Page</div>
            </body>
        </html>
    "#;
    let doc_url = format!("data:text/html,{}", html.replace('\n', "").replace("  ", ""));
    let res = engine.navigate(&doc_url);
    assert!(res.is_ok());
    assert_eq!(engine.current_url(), Some(doc_url.as_str()));

    // Verify background color is green (#00ff00) from the loaded linked stylesheet
    let pixels = engine.pixels();
    assert_eq!(Rgba::new(pixels[0], pixels[1], pixels[2], pixels[3]), Rgba::new(0, 255, 0, 255));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p nirvana_android --test navigate_test`
Expected: FAIL (`navigate` and `current_url` not defined)

- [ ] **Step 3: Implement `navigate`, stylesheet extraction, and JNI bindings**
  - Add `nirvana_net` to `crates/nirvana_android/Cargo.toml`.
  - In `crates/nirvana_android/src/engine.rs`:
    - Add `loader: ResourceLoader` and `current_url: Option<String>` to `NirvanaEngine`.
    - Implement `NirvanaEngine::navigate(&mut self, url_str: &str)`:
      1. Parse `base_url = Url::parse(url_str)`.
      2. Fetch document via `self.loader.fetch(&base_url)`.
      3. Parse HTML and reset scroll coordinates.
      4. Search DOM for `<link>` elements with `rel="stylesheet"` or containing `"stylesheet"` in `rel`.
      5. For each link, extract `href`, resolve against `base_url` via `base_url.join(&href)`.
      6. Fetch stylesheet; if successful, parse via `nirvana_css::parse_css` and add to `self.stylesheets`. If failed, log and continue.
      7. Call `self.render()`.
      8. Set `self.current_url = Some(url_str.to_string())`.
  - In `crates/nirvana_android/src/jni_api.rs`:
    - Add `nativeNavigate` and `nativeGetCurrentUrl` with `catch_unwind` isolation.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p nirvana_android --test navigate_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/nirvana_android
git commit -m "feat(android): implement document navigation and external stylesheet discovery"
```

---

### Task 5: Android Asynchronous Navigation & Auto Link Loading (`NirvanaView.kt`)

**Files:**
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaEngine.kt`
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaView.kt`

**Interfaces:**
- Produces in `NirvanaEngine.kt`:
  - `fun navigate(url: String): Boolean`
  - `val currentUrl: String?`
- Produces in `NirvanaView.kt`:
  - `fun loadUrl(url: String)` (non-blocking, background executor)
  - Default `OnLinkClickListener` behavior navigating to tapped URL

- [ ] **Step 1: Add `navigate` and `currentUrl` to `NirvanaEngine.kt`**
  - Add `private external fun nativeNavigate(enginePtr: Long, url: String): Boolean`
  - Add `private external fun nativeGetCurrentUrl(enginePtr: Long): String?`
  - Implement Kotlin `navigate(url: String): Boolean` and `val currentUrl: String?`.

- [ ] **Step 2: Implement asynchronous `loadUrl` in `NirvanaView.kt`**
  - Add background thread executor: `Executors.newSingleThreadExecutor()`.
  - In `loadUrl(url: String)`:
    - Execute `engine.navigate(url)` on executor thread.
    - On completion, post `drawFrame()` to main thread via `post { drawFrame() }`.
  - In default link tap handling:
    - If `onLinkClickListener` is set, invoke it.
    - If listener is null, automatically call `loadUrl(hit.href)`.

- [ ] **Step 3: Commit**

```bash
git add android/app/src/main/kotlin
git commit -m "feat(android): implement background thread asynchronous loadUrl and link tap navigation"
```

---

### Task 6: End-to-End Navigation Pipeline Test & Full Workspace Verification

**Files:**
- Create: `crates/nirvana_android/tests/navigation_e2e_test.rs`
- Modify: `progress.md`

- [ ] **Step 1: Write comprehensive navigation e2e test**
  - Test navigating between multiple simulated pages (header with links, external CSS styles).
  - Verify clicking link updates URL, document state, and pixel framebuffer.
  - Verify error handling when navigating to invalid/unsupported schemes.

- [ ] **Step 2: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: ALL PASS across all 11 crates.

- [ ] **Step 3: Run Clippy linter**

Run: `cargo clippy --workspace --all-targets`
Expected: 0 warnings.

- [ ] **Step 4: Commit and update progress ledger**

```bash
git add progress.md crates/nirvana_android/tests/navigation_e2e_test.rs
git commit -m "test(android): add end-to-end network navigation integration test"
```
