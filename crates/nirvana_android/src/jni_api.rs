//! JNI C-ABI bindings for Android Java / Kotlin integration.

use crate::engine::NirvanaEngine;
use jni::objects::{JClass, JString};
use jni::sys::{jbyteArray, jint, jlong};
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
