//! JNI C-ABI bindings for Android Java / Kotlin integration.

use crate::engine::NirvanaEngine;
use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jfloat, jfloatArray, jint, jlong, jstring};
use jni::JNIEnv;
use std::panic::catch_unwind;

/// Helper to safely retrieve a mutable reference to `NirvanaEngine` from a `jlong` raw pointer.
/// Returns `None` if the pointer is null.
#[inline]
unsafe fn get_engine_mut<'a>(ptr: jlong) -> Option<&'a mut NirvanaEngine> {
    if ptr == 0 {
        return None;
    }
    unsafe { (ptr as *mut NirvanaEngine).as_mut() }
}

/// Creates a new `NirvanaEngine` instance on the heap and returns its memory address as a `jlong`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) -> jlong {
    let result = catch_unwind(|| {
        let w = width.max(1) as u32;
        let h = height.max(1) as u32;
        let engine = Box::new(NirvanaEngine::new(w, h));
        Box::into_raw(engine) as jlong
    });
    result.unwrap_or(0)
}

/// Destroys a `NirvanaEngine` instance and reclaims its heap memory.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        if engine_ptr != 0 {
            unsafe {
                let _ = Box::from_raw(engine_ptr as *mut NirvanaEngine);
            }
        }
    });
}

/// Loads an HTML string and optional CSS stylesheet into the engine.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeLoadHtml(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    html: JString,
    css: JString,
) {
    let _ = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };

        let html_str: String = match env.get_string(&html) {
            Ok(s) => s.into(),
            Err(_) => return,
        };

        let css_str: Option<String> = if css.is_null() {
            None
        } else {
            env.get_string(&css).ok().map(|s| s.into())
        };

        engine.load_html(&html_str, css_str.as_deref());
    }));
}

/// Resizes the engine's viewport dimensions.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeResize(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    width: jint,
    height: jint,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        let w = width.max(1) as u32;
        let h = height.max(1) as u32;
        engine.resize(w, h);
    });
}

/// Updates the scale factor (e.g. mobile DPR / display density).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeSetScale(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    scale: jfloat,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        engine.set_scale_factor(scale as f32);
    });
}

/// Returns the current scale factor.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetScale(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jfloat {
    catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 1.0;
        };
        engine.scale_factor() as jfloat
    })
    .unwrap_or(1.0)
}

/// Executes the layout and rasterization pass.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeRender(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        engine.render();
    });
}

/// Returns the width of the engine framebuffer in pixels.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetWidth(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jint {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        engine.dimensions().0 as jint
    });
    result.unwrap_or(0)
}

/// Returns the height of the engine framebuffer in pixels.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetHeight(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jint {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        engine.dimensions().1 as jint
    });
    result.unwrap_or(0)
}

/// Copies the 32-bit RGBA pixel buffer into a Java `byte[]`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetPixels(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jbyteArray {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };

        let pixels = engine.pixels();
        match env.byte_array_from_slice(pixels) {
            Ok(arr) => arr.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Copies the 32-bit RGBA pixel buffer directly into a direct `java.nio.ByteBuffer`.
/// Avoids Java heap allocations. Returns 0 on success, negative error code on failure.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeCopyPixelsToBuffer(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    byte_buffer: JByteBuffer,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return -1;
        };
        let target_ptr = match env.get_direct_buffer_address(&byte_buffer) {
            Ok(ptr) if !ptr.is_null() => ptr,
            _ => return -2,
        };
        let capacity = match env.get_direct_buffer_capacity(&byte_buffer) {
            Ok(cap) => cap,
            Err(_) => return -3,
        };

        let pixels = engine.pixels();
        if capacity < pixels.len() {
            return -4;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), target_ptr, pixels.len());
        }
        0
    }));
    result.unwrap_or(-5)
}

/// Scrolls the engine by the given delta (dx, dy).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeScrollBy(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    dx: jfloat,
    dy: jfloat,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        engine.scroll_by(dx, dy);
    });
}

/// Sets the absolute scroll offset of the engine to (x, y).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeScrollTo(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    x: jfloat,
    y: jfloat,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        engine.scroll_to(x, y);
    });
}

/// Returns the current horizontal scroll offset in pixels.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetScrollX(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jfloat {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0.0;
        };
        engine.scroll_offset().0
    });
    result.unwrap_or(0.0)
}

/// Returns the current vertical scroll offset in pixels.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetScrollY(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jfloat {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0.0;
        };
        engine.scroll_offset().1
    });
    result.unwrap_or(0.0)
}

/// Returns the maximum vertical scroll offset in pixels.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetMaxScrollY(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jfloat {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0.0;
        };
        engine.max_scroll().1
    });
    result.unwrap_or(0.0)
}

/// Hit-tests a point on screen. Returns a JSON string describing the target,
/// or an empty string if no target was hit.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeHitTest(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    x: jfloat,
    y: jfloat,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };

        let json = match engine.hit_test(x, y) {
            Some(hit) => {
                let tag = escape_json(&hit.tag);
                let href = hit.href.as_deref().map(escape_json);
                let text = hit.text.as_deref().map(escape_json);
                format!(
                    r#"{{"tag":"{}","href":{},"text":{}}}"#,
                    tag,
                    href.map(|h| format!("\"{}\"", h))
                        .unwrap_or_else(|| "null".to_string()),
                    text.map(|t| format!("\"{}\"", t))
                        .unwrap_or_else(|| "null".to_string()),
                )
            }
            None => String::new(),
        };

        match env.new_string(json) {
            Ok(js) => js.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[cfg(target_os = "android")]
fn log_android(msg: &str) {
    use std::ffi::CString;
    unsafe extern "C" {
        fn __android_log_print(
            prio: std::os::raw::c_int,
            tag: *const std::os::raw::c_char,
            fmt: *const std::os::raw::c_char,
            ...
        ) -> std::os::raw::c_int;
    }
    let tag = CString::new("NirvanaEngine").unwrap_or_default();
    let fmt = CString::new("%s").unwrap_or_default();
    let message = CString::new(msg).unwrap_or_default();
    unsafe {
        __android_log_print(6, tag.as_ptr(), fmt.as_ptr(), message.as_ptr());
    }
}

#[cfg(not(target_os = "android"))]
fn log_android(msg: &str) {
    eprintln!("[NirvanaEngine] {msg}");
}

/// Navigates the engine to a given URL string. Returns true (1) on success, false (0) on failure.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeNavigate(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    url: JString,
) -> jboolean {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        let url_str: String = match env.get_string(&url) {
            Ok(s) => s.into(),
            Err(_) => return 0,
        };
        log_android(&format!("nativeNavigate starting for: {url_str}"));
        match engine.navigate(&url_str) {
            Ok(()) => {
                log_android(&format!("nativeNavigate SUCCESS for: {url_str}"));
                1
            }
            Err(e) => {
                log_android(&format!("nativeNavigate ERROR for {url_str}: {e}"));
                0
            }
        }
    }));
    match result {
        Ok(code) => code,
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic".to_string()
            };
            log_android(&format!("PANIC in nativeNavigate: {msg}"));
            0
        }
    }
}

/// Returns the last navigation error string, or null if no error occurred.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetLastError(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        match engine.last_nav_error.as_deref() {
            Some(err) => match env.new_string(err) {
                Ok(js) => js.into_raw(),
                Err(e) => {
                    log_android(&format!("nativeGetLastError new_string error: {e}"));
                    std::ptr::null_mut()
                }
            },
            None => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Returns the current loaded document URL string, or null if no document is loaded.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetCurrentUrl(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        match engine.current_url() {
            Some(u) => match env.new_string(u) {
                Ok(js) => js.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            None => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Handles a click/touch tap at screen coordinates, returning target JSON and managing input focus.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeHandleClick(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    x: jfloat,
    y: jfloat,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };

        let json = match engine.handle_click(x, y) {
            Some(hit) => {
                let tag = escape_json(&hit.tag);
                let href = hit.href.as_deref().map(escape_json);
                let text = hit.text.as_deref().map(escape_json);
                format!(
                    r#"{{"tag":"{}","href":{},"text":{}}}"#,
                    tag,
                    href.map(|h| format!("\"{}\"", h))
                        .unwrap_or_else(|| "null".to_string()),
                    text.map(|t| format!("\"{}\"", t))
                        .unwrap_or_else(|| "null".to_string()),
                )
            }
            None => String::new(),
        };

        match env.new_string(json) {
            Ok(js) => js.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Returns true (1) if an <input> or <textarea> element currently has focus.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeIsInputFocused(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jboolean {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        if engine.is_input_focused() {
            1
        } else {
            0
        }
    });
    result.unwrap_or(0)
}

/// Inserts text into the currently focused form element.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeInsertText(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    text: JString,
) {
    let _ = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        let text_str: String = match env.get_string(&text) {
            Ok(s) => s.into(),
            Err(_) => return,
        };
        engine.insert_text(&text_str);
    }));
}

/// Deletes the character before the caret in the currently focused form element.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeDeleteBackward(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        engine.delete_backward();
    });
}

/// Returns the current text content of the focused element.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetFocusedText(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        match engine.focused_text() {
            Some(t) => match env.new_string(t) {
                Ok(js) => js.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            None => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Returns the screen bounds [x, y, width, height] of the currently focused element.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetFocusedBounds(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jfloatArray {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        match engine.focused_bounds() {
            Some(rect) => {
                let region = [rect.x(), rect.y(), rect.width(), rect.height()];
                match env.new_float_array(4) {
                    Ok(arr) => {
                        let _ = env.set_float_array_region(&arr, 0, &region);
                        arr.into_raw()
                    }
                    Err(_) => std::ptr::null_mut(),
                }
            }
            None => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Removes focus from currently focused element.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeBlur(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        engine.blur();
    });
}

/// Evaluates a JavaScript script in the engine's active runtime.
/// Returns the evaluation result as a string, or null on error / empty.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeEvaluateJs(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    script: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        let Ok(script_str) = env.get_string(&script) else {
            return std::ptr::null_mut();
        };
        let script_cow = script_str.to_string_lossy();
        match engine.evaluate_js(&script_cow) {
            Ok(val) => match env.new_string(val) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Steps the active JS event loop timers by `delta_ms` and fires callbacks.
/// Returns the number of fired timers.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeStepTimers(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    delta_ms: jlong,
) -> jint {
    let result = catch_unwind(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        let delta = delta_ms.max(0) as u64;
        engine.step_timers(delta) as jint
    });
    result.unwrap_or(0)
}

/// Clears all cookies stored in the engine's sovereign CookieJar.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeClearCookies(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.clear_cookies();
        }
    });
}

/// Clears all local and session Web Storage across all origins.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeClearStorage(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.clear_storage();
        }
    });
}

/// Clears the decoded image cache in the engine.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeClearImageCache(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.clear_image_cache();
        }
    });
}

/// Retrieves high-resolution timing metrics for the most recent render frame.
/// Returns float array: [parse_ms, style_ms, layout_ms, raster_ms, total_ms].
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetFrameMetrics(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jfloatArray {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        let m = engine.frame_metrics();
        let metrics = [
            m.parse_time_ms,
            m.style_time_ms,
            m.layout_time_ms,
            m.raster_time_ms,
            m.total_frame_time_ms,
        ];
        match env.new_float_array(5) {
            Ok(arr) => {
                let _ = env.set_float_array_region(&arr, 0, &metrics);
                arr.into_raw()
            }
            Err(_) => std::ptr::null_mut(),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Returns the total count of blocked ad and tracker requests.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetBlockedCount(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jlong {
    let result = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.blocked_requests_count() as jlong
        } else {
            0
        }
    });
    result.unwrap_or(0)
}

/// Enables or disables sovereign ad blocking.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeSetAdBlockEnabled(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    enabled: jboolean,
) {
    let _ = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.set_adblock_enabled(enabled != 0);
        }
    });
}

/// Checks whether ad blocking is enabled.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeIsAdBlockEnabled(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jboolean {
    let result = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            if engine.is_adblock_enabled() {
                1
            } else {
                0
            }
        } else {
            0
        }
    });
    result.unwrap_or(0)
}

/// Parses and registers a custom AdBlock Plus / uBlock filter rule string.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeAddAdBlockRule(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    rule: JString,
) {
    let _ = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return;
        };
        let Ok(rule_str) = env.get_string(&rule) else {
            return;
        };
        let rule_cow = rule_str.to_string_lossy();
        engine.add_adblock_rule(&rule_cow);
    }));
}

/// Resets the blocked requests counter to zero.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeResetBlockedCount(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    let _ = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.reset_blocked_count();
        }
    });
}

/// Returns the active document `<title>` or null if unavailable.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetPageTitle(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        match engine.page_title() {
            Some(t) => env.new_string(t).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut()),
            None => std::ptr::null_mut(),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

/// Searches rendered document text for `query`, scrolls to `match_index`, and returns match count.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeFindInPage(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    query: JString,
    match_index: jint,
) -> jint {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        let Ok(q_str) = env.get_string(&query) else {
            return 0;
        };
        let q: String = q_str.into();
        engine.find_in_page(&q, match_index.max(0) as usize) as jint
    }))
    .unwrap_or(0)
}

/// Extracts Reader Mode HTML from the active DOM or returns null if empty.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeExtractReaderHtml(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        match engine.extract_reader_html() {
            Some(html) => env.new_string(html).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut()),
            None => std::ptr::null_mut(),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

/// Returns visible text snippet of the current document.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeGetPageTextSnippet(
    env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    max_len: jint,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return std::ptr::null_mut();
        };
        let text = engine.page_text_snippet(max_len.clamp(0, 65536) as usize);
        env.new_string(text).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
    }))
    .unwrap_or(std::ptr::null_mut())
}

/// Configures engine toggles: desktop_mode, https_only, js_enabled, force_dark, dnt.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeConfigure(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    desktop_mode: jboolean,
    https_only: jboolean,
    js_enabled: jboolean,
    force_dark: jboolean,
    do_not_track: jboolean,
) {
    let _ = catch_unwind(|| {
        if let Some(engine) = unsafe { get_engine_mut(engine_ptr) } {
            engine.set_desktop_mode(desktop_mode != 0);
            engine.set_https_only(https_only != 0);
            engine.set_js_enabled(js_enabled != 0);
            engine.set_force_dark_mode(force_dark != 0);
            engine.set_do_not_track(do_not_track != 0);
        }
    });
}

/// Captures a lock-free `NavSnapshot` pointer so network downloading can execute without holding engine locks.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeBeginNavPrefetch(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    url: JString,
) -> jlong {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        let url_str: String = match env.get_string(&url) {
            Ok(s) => s.into(),
            Err(_) => return 0,
        };
        let snap = Box::new(engine.create_nav_snapshot(&url_str));
        Box::into_raw(snap) as jlong
    }))
    .unwrap_or(0)
}

/// Executes HTTP download & concurrent CSS/image/script prefetch using `snapshot_ptr`, freeing `snapshot_ptr`
/// and returning a `Result<PreparedNavigation, String>` pointer (`0` only on panic/null).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeExecuteNavPrefetch(
    mut env: JNIEnv,
    _class: JClass,
    snapshot_ptr: jlong,
    url: JString,
) -> jlong {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        if snapshot_ptr == 0 {
            return 0;
        }
        let snap = unsafe { Box::from_raw(snapshot_ptr as *mut crate::engine::NavSnapshot) };
        let url_str: String = match env.get_string(&url) {
            Ok(s) => s.into(),
            Err(_) => return 0,
        };
        let res = Box::new(crate::engine::fetch_navigation_offline_safe(&url_str, &snap));
        Box::into_raw(res) as jlong
    }))
    .unwrap_or(0)
}

/// Commits (when `apply != 0`) or discards (`apply == 0`) a prefetched `Result<PreparedNavigation, String>` pointer.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nirvana_browser_engine_NirvanaEngine_nativeCommitNavResult(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    result_ptr: jlong,
    apply: jboolean,
) -> jboolean {
    catch_unwind(|| {
        if result_ptr == 0 {
            return 0;
        }
        let boxed = unsafe {
            Box::from_raw(
                result_ptr as *mut Result<crate::engine::PreparedNavigation, String>,
            )
        };
        if apply == 0 {
            return 0;
        }
        let Some(engine) = (unsafe { get_engine_mut(engine_ptr) }) else {
            return 0;
        };
        match *boxed {
            Ok(prepared) => {
                engine.apply_prepared_navigation(prepared);
                1
            }
            Err(e) => {
                engine.last_nav_error = Some(e);
                0
            }
        }
    })
    .unwrap_or(0)
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}
