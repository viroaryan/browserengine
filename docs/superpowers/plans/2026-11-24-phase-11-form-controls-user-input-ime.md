# Phase 11: Form Controls, User Input & Soft Keyboard (IME) Integration

**Status:** IN PROGRESS  
**Date:** 2026-11-24  
**Workspace:** `P:\NirvanaBrowserengine`  
**Goal:** Implement interactive text input controls (`<input type="text">`, `<textarea>`), caret positioning, text editing buffer, touch focus management, and Android soft keyboard (IME) integration via JNI.

---

## 1. Overview & Architectural Motivation

With typography, layout, hit-testing, scrolling, networking, and images fully operational, the engine requires form control interaction. Web browsing is bidirectional: users need to search, type URLs, enter credentials, and interact with web forms.

Phase 11 introduces:
1. **Form Controls User-Agent Styling & Sizing (`nirvana_style`, `nirvana_layout`)**:
   - Default UA rules for `<input>`, `<textarea>`, and `<button>` (borders, padding, background, font).
   - Replaced/inline-block intrinsic geometry (default 160x24px for `<input>`, 200x60px for `<textarea>`).
2. **Text Input State & Editing Buffer (`nirvana_dom`, `nirvana_android`)**:
   - `InputState` managing text value, caret cursor index, and text selection.
   - Text mutation APIs (`insert_text`, `delete_backward`, `set_selection`).
   - Syncing user input to DOM `value` attributes.
3. **Caret & Selection Display List Primitives (`nirvana_gfx`)**:
   - `DisplayCommand::Caret { rect: Rect, color: Rgba }` for active cursor rendering.
   - `DisplayCommand::TextSelection { rect: Rect, color: Rgba }` for highlighted text.
   - Caret x-coordinate calculation via font glyph advances (`nirvana_font`).
4. **Interactive Touch Focus & Hit-Testing (`nirvana_android::NirvanaEngine`)**:
   - Tapping an `<input>` or `<textarea>` focuses the element and computes character caret index from tap coordinates.
   - Tapping outside unfocuses (blurs) the active element.
5. **Android Soft Keyboard (IME) Integration (`NirvanaView.kt`, `jni.rs`)**:
   - JNI bridge methods: `nativeInsertText`, `nativeDeleteBackward`, `nativeIsInputFocused`, `nativeGetFocusedElementBounds`.
   - `onCreateInputConnection(outAttrs: EditorInfo)` returning custom `NirvanaInputConnection`.
   - Automatic `InputMethodManager.showSoftInput` upon focusing input controls and `hideSoftInputFromWindow` upon blurring.

---

## 2. Detailed Task Breakdown

### Task 1: Form Controls UA Styling & Layout Sizing (`nirvana_style`, `nirvana_layout`)
- In `crates/nirvana_style/src/user_agent.rs`:
  - Add default rules for `input`, `textarea`, `button`.
  - Border: `1px solid #767676`, background: white, color: black, padding: `2px 4px`, font-size: `14px`.
- In `crates/nirvana_layout/src/tree_builder.rs` & `block.rs`:
  - Recognize `input` and `textarea` as replaced/inline-block elements with intrinsic sizes:
    - `input`: intrinsic width 160px, height 24px.
    - `textarea`: intrinsic width 200px, height 60px.
- Write unit tests in `crates/nirvana_layout/tests/input_layout_test.rs`.

### Task 2: Caret & Selection Display Commands (`nirvana_gfx`)
- In `crates/nirvana_gfx/src/display_list.rs`:
  - Add `DisplayCommand::Caret { rect: Rect, color: Rgba }`.
  - Add `DisplayCommand::TextSelection { rect: Rect, color: Rgba }`.
- In `crates/nirvana_gfx/src/canvas.rs`:
  - Handle `DisplayCommand::Caret` and `DisplayCommand::TextSelection`.
- In `crates/nirvana_gfx/src/builder.rs`:
  - For focused form controls, measure text width using `nirvana_font`, emit inner text / placeholder, emit caret bar at cursor x-offset.
- Write unit tests in `crates/nirvana_gfx/tests/caret_paint_test.rs`.

### Task 3: Input State Management & Engine Editing APIs (`nirvana_android`)
- In `crates/nirvana_android/src/engine.rs`:
  - Struct `InputState { text: String, cursor_index: usize, selection: Option<(usize, usize)> }`.
  - Field `focused_element: Option<NodeId>`.
  - Field `inputs: HashMap<NodeId, InputState>`.
  - Methods:
    - `focus(node_id: NodeId, cursor_index: usize)`
    - `blur()`
    - `insert_text(text: &str)`
    - `delete_backward()`
    - `focused_text() -> Option<String>`
    - `focused_bounds() -> Option<Rect>`
    - `is_input_focused() -> bool`
- Update `handle_click(x, y)`:
  - If target is `<input>` or `<textarea>`, compute caret index from relative click `x`, set focus.
  - If target is not a form control, blur.
- Write unit tests in `crates/nirvana_android/tests/engine_input_test.rs`.

### Task 4: JNI Bridge for Input & IME (`crates/nirvana_android/src/jni.rs`)
- Add JNI exports:
  - `nativeInsertText(ptr, text)`
  - `nativeDeleteBackward(ptr)`
  - `nativeIsInputFocused(ptr) -> jboolean`
  - `nativeGetFocusedElementBounds(ptr) -> jfloatArray`
- Write unit tests in `crates/nirvana_android/tests/jni_input_test.rs`.

### Task 5: Android `NirvanaView.kt` Soft Keyboard Integration
- In `crates/nirvana_android/src/android/NirvanaView.kt`:
  - Override `onCreateInputConnection(outAttrs: EditorInfo): InputConnection?`.
  - Implement `NirvanaInputConnection : BaseInputConnection`:
    - `commitText(text: CharSequence?, newCursorPosition: Int): Boolean`
    - `deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean`
    - `sendKeyEvent(event: KeyEvent?): Boolean`
  - In `onTouchEvent`:
    - After hit-test, if input is focused, call `imm.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT)`.
    - If no input is focused, call `imm.hideSoftInputFromWindow(windowToken, 0)`.

### Task 6: End-to-End User Input Test & Full Workspace Verification
- In `crates/nirvana_android/tests/input_ime_e2e_test.rs`:
  - Load HTML with `<input type="text" value="hello" placeholder="Search...">`.
  - Tap into input at character boundary, verify caret position.
  - Type text via `insert_text(" world")`, verify updated value "hello world" and re-rendered pixels.
  - Delete backward, verify caret and value update.
  - Tap outside, verify blur.
- Run `cargo test --workspace` across all crates (100% passing).
- Run `cargo clippy --workspace --all-targets` (0 warnings).
- Strict `#![forbid(unsafe_code)]` compliance.
- Independent code reviewer sign-off.
