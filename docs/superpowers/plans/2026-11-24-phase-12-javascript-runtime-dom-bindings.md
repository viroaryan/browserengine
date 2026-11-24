# Phase 12: JavaScript Runtime Integration & Sovereign DOM Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a sovereign, pure-Rust JavaScript runtime (`crates/nirvana_js`) powered by `boa_engine`, implement safe two-way DOM bindings (`document.getElementById`, `innerHTML`, `textContent`, `setAttribute`, `style`), wire an event listener system (`addEventListener('click', ...)`), timer event loop (`setTimeout`), and integrate `<script>` execution and dynamic re-rendering into `nirvana_android::NirvanaEngine`.

**Architecture:** A dedicated crate `nirvana_js` provides `JsRuntime` wrapping `boa_engine::Context`. A shared DOM reference (`Rc<RefCell<DomTree>>`) bridges JavaScript `document` and `Element` host objects directly to the Sovereign DOM. When scripts modify the DOM or change styles, mutation flags trigger layout and paint updates in `NirvanaEngine`. Touch taps and keyboard inputs dispatch DOM events (`click`, `input`) to registered JavaScript callbacks.

**Tech Stack:** Rust (Edition 2024 / 1.85+ compatible), `boa_engine = "0.22.0"`, `nirvana_core`, `nirvana_dom`, `nirvana_html`, `nirvana_css`, `nirvana_style`, `nirvana_layout`, `nirvana_gfx`, `nirvana_net`, `nirvana_android`, `#![forbid(unsafe_code)]`.

**Spec:** `docs/superpowers/specs/2026-11-23-nirvana-engine-design.md`

## Global Constraints

- Pure Rust `#![forbid(unsafe_code)]` in all 11 core crates including `crates/nirvana_js`.
- Unsafe code strictly limited to JNI boundary in `nirvana_android::jni_api`, guarded by `std::panic::catch_unwind` and null pointer checks.
- Zero C/C++ compiler (`cl.exe`) dependencies on Windows host.
- TDD Red-Green-Refactor with independent adversarial code reviewer subagents.
- All workspace tests must pass (`cargo test --workspace`), 0 clippy warnings (`cargo clippy --workspace --all-targets`).

## Review Focus

1. **DOM Mutation Memory Safety & Reentrancy:** Ensuring `DomTree` updates from JS do not cause borrow panics when layout or hit-testing queries DOM state.
2. **Infinite Loops & Script Execution Limits:** Scripts that throw errors or encounter uncaught exceptions must be captured gracefully without panicking the browser engine.
3. **Event Listener Bubble & Invocation Order:** Dispatching `'click'` events from target element upward through parent chain (bubbling).
4. **Dynamic Re-layout & Re-render Synchronization:** When JS modifies `innerHTML`, `textContent`, or `style`, `NirvanaEngine` must automatically invalidate layout and re-render without manual reload.
5. **Memory Management of JS Callbacks:** Ensuring event listener closures and timer callbacks are tracked and cleaned up when documents are reloaded.

---

### Task 1: `crates/nirvana_js` Scaffolding & Pure-Rust JS Runtime Core

**Files:**
- Create: `crates/nirvana_js/Cargo.toml`
- Create: `crates/nirvana_js/src/lib.rs`
- Create: `crates/nirvana_js/src/runtime.rs`
- Create: `crates/nirvana_js/src/console.rs`
- Modify: `Cargo.toml:1-25` (add `crates/nirvana_js` to workspace members)
- Test: `crates/nirvana_js/tests/runtime_test.rs`

- [ ] **Step 1.1: Write failing test in `runtime_test.rs`**
  - Test initializing `JsRuntime::new()`.
  - Test `eval("1 + 2")` returns `"3"`.
  - Test `console.log("hello", 42)` captures log entries in `runtime.logs()`.
  - Test syntax error in `eval` returns `Err(JsError)`.
- [ ] **Step 1.2: Implement `crates/nirvana_js/Cargo.toml` and workspace configuration**
  - Add `boa_engine = "0.22.0"`, `nirvana_core`, `nirvana_dom`.
  - Add `#![forbid(unsafe_code)]` at `src/lib.rs`.
- [ ] **Step 1.3: Implement `console.rs`**
  - Register global `console` object with `.log()`, `.warn()`, `.error()`, `.info()`.
  - Store messages with log levels and formatted strings into thread-safe or `Rc<RefCell<Vec<LogEntry>>>`.
- [ ] **Step 1.4: Implement `runtime.rs`**
  - Wrap `boa_engine::Context` into `JsRuntime`.
  - Implement `pub fn eval(&mut self, code: &str) -> Result<String, String>`.
  - Implement `pub fn logs(&self) -> Vec<LogEntry>`.
- [ ] **Step 1.5: Run tests and verify pass**
  - Run `cargo test -p nirvana_js` and confirm all tests pass.
- [ ] **Step 1.6: Commit Task 1**
  - `git add Cargo.toml crates/nirvana_js ; git commit -m "feat(js): scaffold nirvana_js crate and core JsRuntime with console logging"`

---

### Task 2: Sovereign DOM Element Object & Document Bindings

**Files:**
- Create: `crates/nirvana_js/src/dom_binding.rs`
- Modify: `crates/nirvana_js/src/runtime.rs`
- Modify: `crates/nirvana_js/src/lib.rs`
- Test: `crates/nirvana_js/tests/dom_bindings_test.rs`

- [ ] **Step 2.1: Write failing test in `dom_bindings_test.rs`**
  - Create a `DomTree` with `<div id="greeting" class="box" style="color: red;">Hello <b>World</b></div>`.
  - Bind DOM to `JsRuntime::with_dom(dom)`.
  - Test `document.getElementById("greeting") != null`.
  - Test `document.getElementById("greeting").textContent` returns `"Hello World"`.
  - Test setting `elem.textContent = "New Text"` mutates DOM text nodes and sets mutation flag.
  - Test `elem.getAttribute("class")` returns `"box"`.
  - Test `elem.setAttribute("title", "tooltip")` mutates DOM attribute.
  - Test `elem.innerHTML = "<span>Dynamic</span>"` parses and replaces DOM children.
  - Test `elem.style.color = "blue"` updates element `style` attribute.
- [ ] **Step 2.2: Implement `dom_binding.rs`**
  - Define `SharedDom = Rc<RefCell<DomTree>>`.
  - Register global `document` object with native methods:
    - `getElementById(id: String) -> JsValue`
    - `querySelector(selector: String) -> JsValue`
    - `createElement(tag: String) -> JsValue`
  - Define `Element` class in JS context with prototype accessors:
    - `get textContent()`, `set textContent(val)`
    - `get innerHTML()`, `set innerHTML(val)` (using `nirvana_html::parse_html`)
    - `getAttribute(name)`, `setAttribute(name, value)`
    - `get value()`, `set value(val)` (for `<input>` and `<textarea>`)
    - `style` proxy / helper for style properties.
- [ ] **Step 2.3: Implement mutation tracking**
  - Track `has_mutated: bool` in `JsRuntime` so engine knows when layout/paint invalidation is needed.
- [ ] **Step 2.4: Run tests and verify pass**
  - Run `cargo test -p nirvana_js --test dom_bindings_test` and confirm all tests pass.
- [ ] **Step 2.5: Commit Task 2**
  - `git add crates/nirvana_js ; git commit -m "feat(js): implement sovereign DOM bindings for document and Element"`

---

### Task 3: DOM Events & Event Listeners (`addEventListener`, `dispatchEvent`)

**Files:**
- Create: `crates/nirvana_js/src/events.rs`
- Modify: `crates/nirvana_js/src/dom_binding.rs`
- Modify: `crates/nirvana_js/src/runtime.rs`
- Test: `crates/nirvana_js/tests/events_test.rs`

- [ ] **Step 3.1: Write failing test in `events_test.rs`**
  - Test `element.addEventListener("click", function(e) { ... })`.
  - Test dispatching click event from Rust: `runtime.dispatch_event(node_id, "click")`.
  - Verify JS callback executes and updates JS/DOM state.
  - Test event bubbling from child to parent element.
  - Test multiple listeners on the same element.
  - Test `removeEventListener` removes registered listener.
- [ ] **Step 3.2: Implement `events.rs`**
  - Define `EventListener` storing `JsFunction`.
  - Registry in `JsRuntime`: `listeners: HashMap<(NodeId, String), Vec<JsFunction>>`.
  - Expose `addEventListener(type, callback)` and `removeEventListener(type, callback)` on `Element` prototype.
  - Implement `dispatch_event(&mut self, target_node: NodeId, event_name: &str, bubbles: bool) -> Result<(), String>`.
  - Implement bubbling: walk up `dom.parent(curr_id)` invoking listeners registered on ancestors.
- [ ] **Step 3.3: Run tests and verify pass**
  - Run `cargo test -p nirvana_js --test events_test` and confirm all tests pass.
- [ ] **Step 3.4: Commit Task 3**
  - `git add crates/nirvana_js ; git commit -m "feat(js): implement addEventListener and DOM event bubbling dispatch"`

---

### Task 4: Timers & Event Loop (`setTimeout`, `clearTimeout`)

**Files:**
- Create: `crates/nirvana_js/src/timers.rs`
- Modify: `crates/nirvana_js/src/runtime.rs`
- Test: `crates/nirvana_js/tests/timer_test.rs`

- [ ] **Step 4.1: Write failing test in `timer_test.rs`**
  - Test `setTimeout(callback, 100)` registers timer and returns integer timer ID.
  - Test `runtime.step_timers(50)` does not fire timer.
  - Test `runtime.step_timers(60)` (total 110ms) fires timer callback.
  - Test `clearTimeout(id)` cancels timer before firing.
- [ ] **Step 4.2: Implement `timers.rs`**
  - Store `Timer` struct: `{ id: u32, callback: JsFunction, remaining_ms: u64 }`.
  - Expose `setTimeout` and `clearTimeout` on global object.
  - Implement `step_timers(&mut self, delta_ms: u64) -> usize`: decrements remaining time, collects expired callbacks, executes them, and returns number of fired timers.
- [ ] **Step 4.3: Run tests and verify pass**
  - Run `cargo test -p nirvana_js --test timer_test` and confirm all tests pass.
- [ ] **Step 4.4: Commit Task 4**
  - `git add crates/nirvana_js ; git commit -m "feat(js): implement setTimeout, clearTimeout, and timer event loop"`

---

### Task 5: Engine `<script>` Lifecycle, DOM Events & JNI Integration (`nirvana_android`)

**Files:**
- Modify: `crates/nirvana_android/Cargo.toml` (depend on `nirvana_js`)
- Modify: `crates/nirvana_android/src/engine.rs`
- Modify: `crates/nirvana_android/src/jni_api.rs`
- Modify: `android/app/src/main/kotlin/com/nirvana/browser/engine/NirvanaEngine.kt`
- Test: `crates/nirvana_android/tests/engine_js_test.rs`

- [ ] **Step 5.1: Write failing test in `engine_js_test.rs`**
  - Load HTML with `<h1 id="title">Old Title</h1><script>document.getElementById("title").textContent = "New Title";</script>`.
  - Verify rendered document displays "New Title".
  - Test script modifying style: `<div id="box" style="width: 100px;"></div><script>document.getElementById("box").setAttribute("style", "width: 200px;"); document.getElementById("box").style.color = "red";</script>`.
  - Verify layout box width updates to 200px after script run.
  - Test clicking an element with a `'click'` listener invokes the callback.
  - Test `engine.evaluate_js("document.title = 'Updated Title'")`.
- [ ] **Step 5.2: Integrate `JsRuntime` into `NirvanaEngine`**
  - Add `js_runtime: Option<JsRuntime>` to `NirvanaEngine`.
  - In `load_html`:
    - After DOM parse, create `SharedDom = Rc::new(RefCell::new(dom))`.
    - Initialize `JsRuntime::with_dom(shared_dom)`.
    - Traverse DOM tree for `<script>` elements.
    - For each `<script>` without `src`: extract text content and `js_runtime.eval(&script_text)`.
    - For `<script src="...">`: fetch content via `ResourceLoader` and execute.
    - Check `js_runtime.has_mutated()`: if true, re-compute styles and re-layout.
- [ ] **Step 5.3: Wire interactive events into `handle_click` and `insert_text`**
  - In `handle_click(x, y)`:
    - Dispatch `'click'` event to `hit.node_id` via `js_runtime.dispatch_event`.
    - If DOM was mutated by the click handler, re-compute styles, layout, and render.
  - In `insert_text` and `delete_backward`:
    - Dispatch `'input'` event to `focused_element`.
- [ ] **Step 5.4: Wire JNI and Kotlin bindings**
  - In `jni_api.rs`: add `Java_com_nirvana_browser_engine_NirvanaEngine_nativeEvaluateJs(ptr, script) -> jstring`.
  - In `NirvanaEngine.kt`: expose `fun evaluateJs(script: String): String?`.
- [ ] **Step 5.5: Run tests and verify pass**
  - Run `cargo test -p nirvana_android --test engine_js_test` and confirm all tests pass.
- [ ] **Step 5.6: Commit Task 5**
  - `git add crates/nirvana_android android/ ; git commit -m "feat(engine): wire JsRuntime into NirvanaEngine with script execution and click event dispatch"`

---

### Task 6: End-to-End Dynamic Web App Test & Full Workspace Verification

**Files:**
- Create: `crates/nirvana_android/tests/js_dynamic_e2e_test.rs`
- Modify: `progress.md`
- Test: `cargo test --workspace`, `cargo clippy --workspace --all-targets`

- [ ] **Step 6.1: Write and run end-to-end integration test `js_dynamic_e2e_test.rs`**
  - Load a complete dynamic interactive web app:
    ```html
    <html>
      <body>
        <h1 id="header">Counter App</h1>
        <button id="counter-btn">Count: 0</button>
        <div id="output">Initial</div>
        <script>
          var count = 0;
          var btn = document.getElementById('counter-btn');
          var out = document.getElementById('output');
          btn.addEventListener('click', function() {
            count++;
            btn.textContent = 'Count: ' + count;
            out.innerHTML = '<b>Updated ' + count + '</b>';
          });
        </script>
      </body>
    </html>
    ```
  - Verify initial layout and rasterization contains "Count: 0".
  - Simulate click on `#counter-btn`:
    - Verify click handler fires, count increments to 1.
    - Verify button text updates to "Count: 1".
    - Verify `#output` innerHTML updates to child `<b>` tag containing "Updated 1".
    - Verify display list and framebuffer re-render with new text and layout coordinates.
  - Simulate second click: count increments to 2, verifying continuous reactivity.
- [ ] **Step 6.2: Full workspace quality & safety check**
  - Run `cargo test --workspace` across all 12 crates (100% passing).
  - Run `cargo clippy --workspace --all-targets` (0 warnings).
  - Verify `#![forbid(unsafe_code)]` in all 11 core crates.
- [ ] **Step 6.3: Independent Code Review**
  - Spawn an adversarial Senior Browser Engine & Language Runtime Code Reviewer subagent to review the entire Phase 12 diff.
  - Remediate any findings until `[APPROVAL]` is obtained.
- [ ] **Step 6.4: Update `progress.md` and sign off Phase 12**
  - Mark Phase 12 as COMPLETED & SIGNED OFF in `progress.md`.
  - Commit: `git add progress.md ; git commit -m "docs: mark Phase 12 complete and signed off"`
