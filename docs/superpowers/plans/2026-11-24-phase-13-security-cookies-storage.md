# Phase 13: Web Security, Cookie Management, Storage & HTTPS TLS Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a sovereign, pure-Rust Web Security, RFC 6265 Cookie Management, Web Storage (`localStorage`/`sessionStorage`), and HTTPS/TLS hardening subsystem for the Nirvana Browser Engine.

**Architecture:** Implement `CookieJar` in `nirvana_net` with RFC 6265 parsing, domain/path matching, and `HttpOnly`/`Secure`/`SameSite` semantics; origin-based Same-Origin Policy (SOP) and mixed-content enforcement; Web Storage API (`localStorage`/`sessionStorage`) in `nirvana_js`; `document.cookie` getter/setter bindings with `HttpOnly` isolation; and Android JNI cookie/storage management APIs.

**Tech Stack:** Pure Rust, `ureq` (with rustls), `boa_engine` (v0.22.0), `nirvana_url`, JNI C-ABI, Kotlin.

**Spec:** Sovereign Browser Engine Architecture Specification — Phase 13.

## Global Constraints

- Strict `#![forbid(unsafe_code)]` across all core crates (`nirvana_net`, `nirvana_js`, `nirvana_core`, `nirvana_dom`, `nirvana_url`).
- Unsafe code strictly confined to JNI FFI boundary in `nirvana_android::jni_api`, guarded by `std::panic::catch_unwind` and pointer validation.
- Zero C/C++ compiler (`cl.exe`) dependencies on Windows host.
- Pure Rust TLS using `rustls` through `ureq`.
- TDD Red-Green-Refactor with independent code review per phase.

## Review Focus

1. **RFC 6265 Cookie Matching Invariants:** Domain and path matching rules (`domain_matches`, `path_matches`) must strictly prevent cross-domain cookie leakage (e.g. cookie for `sub.example.com` must not leak to `other.example.com`, and super-domain cookies must follow public suffix / dot-prefix boundaries).
2. **`HttpOnly` Cookie Security Boundary:** Cookies flagged with `HttpOnly` must be transmitted over HTTP/HTTPS network requests but must NEVER be accessible, readable, or writable via `document.cookie` in JavaScript.
3. **Web Storage Origin Isolation:** `localStorage` and `sessionStorage` must be partitioned strictly by `Origin` (`scheme://host:port`). Scripts running on `https://site-a.com` must never read or modify storage of `https://site-b.com`.
4. **Mixed Content Detection:** Subresources fetched over insecure `http://` from a secure `https://` origin must be detected and blocked/flagged to prevent SSL-strip and MitM tampering.
5. **Reentrancy and Concurrency Safety:** `CookieJar` and `Storage` shared across network loader, DOM bindings, and JavaScript runtime must avoid deadlock or RefCell borrowing panics during concurrent resource fetches and DOM execution.

---

### Task 1: RFC 6265 Cookie Model, Parser & Sovereign CookieJar (`nirvana_net`)

**Files:**
- Create: `crates/nirvana_net/src/cookie.rs`
- Modify: `crates/nirvana_net/src/lib.rs`
- Modify: `crates/nirvana_net/src/response.rs`
- Test: `crates/nirvana_net/tests/cookie_test.rs`

- [ ] **Step 1.1: Write failing tests for Cookie model and parsing**
  Create `crates/nirvana_net/tests/cookie_test.rs` testing:
  - `parse_set_cookie` parsing `name=value; Path=/; Domain=example.com; Secure; HttpOnly; SameSite=Lax`.
  - Expiry and max-age parsing (`Max-Age=3600`).
  - Domain matching (`example.com` matches `example.com` and `sub.example.com`, but not `other.com`).
  - Path matching (`/docs` matches `/docs` and `/docs/api`, but not `/img`).
  - CookieJar storing, retrieving matching cookies, and generating `Cookie` header string.

- [ ] **Step 1.2: Run test to confirm failure**
  Run `cargo test -p nirvana_net --test cookie_test`.

- [ ] **Step 1.3: Implement Cookie and CookieJar in `crates/nirvana_net/src/cookie.rs`**
  - Implement `SameSite` enum (`Strict`, `Lax`, `None`).
  - Implement `Cookie` struct with RFC 6265 attributes.
  - Implement `parse_set_cookie(header: &str, default_domain: &str, default_path: &str) -> Option<Cookie>`.
  - Implement `CookieJar`:
    - `store(&mut self, cookie: Cookie, origin: &Url)`
    - `get_cookies_for_url(&self, url: &Url, is_http: bool) -> Vec<&Cookie>`
    - `get_cookie_header_value(&self, url: &Url, is_http: bool) -> Option<String>`
    - `clear(&mut self)`
  - Update `crates/nirvana_net/src/response.rs`: add `pub raw_headers: Vec<(String, String)>` and `pub fn get_all_headers(&self, name: &str) -> Vec<&str>` to preserve multiple `Set-Cookie` headers.
  - Export in `crates/nirvana_net/src/lib.rs`.

- [ ] **Step 1.4: Run tests to verify pass**
  Run `cargo test -p nirvana_net --test cookie_test`.

- [ ] **Step 1.5: Commit Task 1**
  `git add crates/nirvana_net/ ; git commit -m "feat(net): implement RFC 6265 Cookie model, parser, and CookieJar"`

---

### Task 2: Origin Representation, Same-Origin Policy & Mixed Content Detection (`nirvana_net`)

**Files:**
- Create: `crates/nirvana_net/src/security.rs`
- Modify: `crates/nirvana_net/src/lib.rs`
- Test: `crates/nirvana_net/tests/security_test.rs`

- [ ] **Step 2.1: Write failing tests for Origin, SOP, and Mixed Content**
  Create `crates/nirvana_net/tests/security_test.rs` testing:
  - `Origin::from_url` correctly extracts scheme, host, port (default 80 for http, 443 for https).
  - `Origin::is_same_origin` verifies scheme, host, and port equality.
  - `is_mixed_content(doc_origin, subresource_url)`:
    - `https://secure.com` requesting `http://insecure.com/script.js` returns `true` (Mixed Active Content).
    - `https://secure.com` requesting `https://cdn.com/style.css` returns `false`.
    - `http://insecure.com` requesting `http://insecure.com/img.png` returns `false`.
  - `CspPolicy` parsing basic directives: `default-src 'self'; img-src https: data:`.

- [ ] **Step 2.2: Run test to confirm failure**
  Run `cargo test -p nirvana_net --test security_test`.

- [ ] **Step 2.3: Implement Origin, Mixed Content, and CSP in `crates/nirvana_net/src/security.rs`**
  - Implement `Origin` struct with `from_url`, `is_same_origin`, and `to_string`.
  - Implement `is_mixed_content(doc_origin: &Origin, subresource_url: &Url) -> bool`.
  - Implement `CspPolicy` and `CspDirective` for script/style/img source validation.
  - Export in `crates/nirvana_net/src/lib.rs`.

- [ ] **Step 2.4: Run tests to verify pass**
  Run `cargo test -p nirvana_net --test security_test`.

- [ ] **Step 2.5: Commit Task 2**
  `git add crates/nirvana_net/ ; git commit -m "feat(net): implement Origin, Same-Origin Policy, Mixed Content, and CSP checks"`

---

### Task 3: Web Storage API (`localStorage` & `sessionStorage` in `nirvana_js`)

**Files:**
- Create: `crates/nirvana_js/src/storage.rs`
- Modify: `crates/nirvana_js/src/lib.rs`
- Modify: `crates/nirvana_js/src/runtime.rs`
- Test: `crates/nirvana_js/tests/storage_test.rs`

- [ ] **Step 3.1: Write failing tests for Web Storage**
  Create `crates/nirvana_js/tests/storage_test.rs` testing:
  - `window.localStorage.setItem('key', 'value')`, `getItem('key')`, `removeItem('key')`, `clear()`, `length`, `key(0)`.
  - `window.sessionStorage` operates independently of `localStorage`.
  - Origin isolation: switching runtime origin isolates stored keys between origins.

- [ ] **Step 3.2: Run test to confirm failure**
  Run `cargo test -p nirvana_js --test storage_test`.

- [ ] **Step 3.3: Implement Web Storage in `crates/nirvana_js/src/storage.rs`**
  - Implement `StorageArea`: thread-safe / `Rc<RefCell<HashMap<String, String>>>`.
  - Implement `StorageBackend`: `HashMap<String, StorageArea>` indexed by origin string (`scheme://host:port`).
  - Implement `create_storage_object(context, storage_area) -> JsObject`.
  - Register `localStorage` and `sessionStorage` on `window` in `JsRuntime`.
  - Add origin parameter to `JsRuntime` initialization or navigation: `set_origin(&mut self, origin: &str)`.

- [ ] **Step 3.4: Run tests to verify pass**
  Run `cargo test -p nirvana_js --test storage_test`.

- [ ] **Step 3.5: Commit Task 3**
  `git add crates/nirvana_js/ ; git commit -m "feat(js): implement localStorage and sessionStorage Web Storage APIs with origin isolation"`

---

### Task 4: `document.cookie` DOM Bindings with `HttpOnly` Isolation (`nirvana_js`)

**Files:**
- Modify: `crates/nirvana_js/src/dom_binding.rs`
- Modify: `crates/nirvana_js/src/runtime.rs`
- Test: `crates/nirvana_js/tests/document_cookie_test.rs`

- [ ] **Step 4.1: Write failing tests for `document.cookie`**
  Create `crates/nirvana_js/tests/document_cookie_test.rs` testing:
  - Reading `document.cookie` returns non-`HttpOnly` cookies formatted as `"k1=v1; k2=v2"`.
  - `HttpOnly` cookies in the jar are invisible to `document.cookie`.
  - Setting `document.cookie = "theme=dark; path=/"` updates the `CookieJar` for current URL.
  - Setting a cookie with `HttpOnly` via JavaScript does not set `HttpOnly` (or is ignored per spec).

- [ ] **Step 4.2: Run test to confirm failure**
  Run `cargo test -p nirvana_js --test document_cookie_test`.

- [ ] **Step 4.3: Implement `document.cookie` getter and setter**
  - In `crates/nirvana_js/src/dom_binding.rs`:
    - Add `SharedCookieJar` (`Rc<RefCell<CookieJar>>`) and document URL to `DomHandle`.
    - Register accessor `document.cookie` on `document` object.
    - Getter: queries `cookie_jar.borrow().get_cookies_for_url(&url, false)` (non-HTTP), filters out `http_only`, formats as `"k1=v1; k2=v2"`.
    - Setter: parses cookie string, sets default domain/path from document URL, ensures `http_only = false`, stores into `cookie_jar`.

- [ ] **Step 4.4: Run tests to verify pass**
  Run `cargo test -p nirvana_js --test document_cookie_test`.

- [ ] **Step 4.5: Commit Task 4**
  `git add crates/nirvana_js/ ; git commit -m "feat(js): implement document.cookie DOM bindings with HttpOnly security isolation"`

---

### Task 5: Engine Pipeline Integration, CookieJar Wiring & JNI Android Bindings (`nirvana_android`)

**Files:**
- Modify: `crates/nirvana_android/src/engine.rs`
- Modify: `crates/nirvana_android/src/jni_api.rs`
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaEngine.kt`
- Test: `crates/nirvana_android/tests/engine_security_test.rs`

- [ ] **Step 5.1: Write failing tests for Engine Security & Cookie pipeline**
  Create `crates/nirvana_android/tests/engine_security_test.rs` testing:
  - Engine retains `CookieJar` across navigations.
  - `engine.clear_cookies()` empties the jar.
  - `engine.clear_storage()` clears local and session storage.
  - Insecure subresource blocking on HTTPS documents.

- [ ] **Step 5.2: Run test to confirm failure**
  Run `cargo test -p nirvana_android --test engine_security_test`.

- [ ] **Step 5.3: Integrate CookieJar and Storage in `NirvanaEngine` and JNI**
  - Add `cookie_jar: std::sync::Arc<std::sync::Mutex<CookieJar>>` and `storage_backend: std::sync::Arc<std::sync::Mutex<StorageBackend>>` in `NirvanaEngine`.
  - Pass cookie jar to `ResourceLoader` and `JsRuntime`.
  - Implement `clear_cookies()` and `clear_storage()` on `NirvanaEngine`.
  - Add JNI functions in `crates/nirvana_android/src/jni_api.rs`:
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeClearCookies`
    - `Java_com_nirvana_browser_engine_NirvanaEngine_nativeClearStorage`
  - In `NirvanaEngine.kt`, expose `fun clearCookies()` and `fun clearStorage()`.

- [ ] **Step 5.4: Run tests to verify pass**
  Run `cargo test -p nirvana_android --test engine_security_test`.

- [ ] **Step 5.5: Commit Task 5**
  `git add crates/nirvana_android/ android/ ; git commit -m "feat(engine): wire CookieJar, Web Storage, and JNI clear APIs into NirvanaEngine"`

---

### Task 6: End-to-End Security Integration Tests & Full Workspace Verification

**Files:**
- Create: `crates/nirvana_android/tests/security_e2e_test.rs`
- Modify: `progress.md`

- [ ] **Step 6.1: Write comprehensive end-to-end security integration tests**
  Create `crates/nirvana_android/tests/security_e2e_test.rs`:
  - E2E Web App with login state simulated via `document.cookie` and `localStorage`.
  - Form submit sets session token in `localStorage`, updates status in DOM.
  - Verifying origin isolation between two different origins.
  - Verifying `HttpOnly` protection.

- [ ] **Step 6.2: Run full workspace test suite**
  Run `cargo test --workspace` across all crates.

- [ ] **Step 6.3: Run workspace linter**
  Run `cargo clippy --workspace --all-targets` (must be 0 warnings).

- [ ] **Step 6.4: Verify `#![forbid(unsafe_code)]` compliance**
  Ensure all core crates enforce `#![forbid(unsafe_code)]`.

- [ ] **Step 6.5: Independent Code Review & Final Sign-Off**
  - Generate diff for Phase 13.
  - Invoke Senior Browser Security & Network Code Reviewer subagent.
  - Address any findings until `[APPROVAL]` is obtained.
  - Update `progress.md` and commit.
