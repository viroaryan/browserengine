//! JNI C-ABI bindings for Android Java / Kotlin integration.

use crate::engine::NirvanaEngine;
use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jfloat, jint, jlong, jstring};
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
        if engine.navigate(&url_str).is_ok() {
            1
        } else {
            0
        }
    }));
    result.unwrap_or(0)
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
