# Nirvana Browser Engine — Android Integration Guide

This guide describes how to integrate the Nirvana Browser Engine into an Android application to replace GeckoView or Android WebView.

---

## 1. Project Setup & Native Libraries

### 1.1. Native Shared Libraries (`jniLibs`)

Compile `nirvana_android` for target Android ABIs:

```bash
# Example: Building for ARM64 (standard for modern Android devices)
rustup target add aarch64-linux-android
cargo build -p nirvana_android --release --target aarch64-linux-android
```

Copy the generated shared library into your Android project:
```
app/
└── src/
    └── main/
        └── jniLibs/
            └── arm64-v8a/
                └── libnirvana_engine.so
```

### 1.2. Kotlin Engine Source Files

Include the engine wrapper classes in your app:
- `com.nirvana.browser.engine.NirvanaEngine`
- `com.nirvana.browser.engine.NirvanaView`
- `com.nirvana.browser.engine.HitTarget`
- `com.nirvana.browser.engine.FrameMetrics`

---

## 2. Using `NirvanaView` in Activities / Fragments

`NirvanaView` is a complete Android `View` component that handles canvas drawing, touch hit testing, smooth scrolling, and Android virtual keyboard (IME) input.

### 2.1. Adding to XML Layout

```xml
<?xml version="1.0" encoding="utf-8"?>
<FrameLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:layout_width="match_parent"
    android:layout_height="match_parent">

    <com.nirvana.browser.engine.NirvanaView
        android:id="@+id/browserView"
        android:layout_width="match_parent"
        android:layout_height="match_parent" />

</FrameLayout>
```

### 2.2. Loading Content in Kotlin

```kotlin
class BrowserActivity : AppCompatActivity() {

    private lateinit var browserView: NirvanaView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_browser)

        browserView = findViewById(R.id.browserView)

        // Set up interactive click listener
        browserView.onLinkClickListener = { url ->
            println("User tapped link: $url")
            browserView.engine.navigate(url)
        }

        // Load an initial HTML document or navigate to a URL
        browserView.loadHtml(
            html = "<h1>Nirvana Browser</h1><p>Running on pure Rust engine.</p>",
            css = "h1 { color: #1a73e8; } body { padding: 16px; }"
        )
    }

    override fun onDestroy() {
        super.onDestroy()
        // NirvanaView automatically manages engine lifecycle,
        // but explicit cleanup can be invoked here:
        browserView.engine.clearImageCache()
    }
}
```

---

## 3. Direct Programmatic Engine Usage (`NirvanaEngine`)

If you want headless rendering, offscreen capture, or custom surface drawing:

```kotlin
// 1. Create engine instance
val engine = NirvanaEngine(width = 1080, height = 1920)

// 2. Load web content
engine.loadHtml("<div><h1>Offscreen Page</h1></div>")

// 3. Render frame
engine.render()

// 4. Retrieve raw 32-bit RGBA byte buffer
val pixels: ByteArray = engine.getPixels()

// 5. Or copy directly into an Android direct ByteBuffer
val byteBuffer = ByteBuffer.allocateDirect(1080 * 1920 * 4)
engine.copyPixelsToBuffer(byteBuffer)

// 6. Inspect rendering performance telemetry
val metrics = engine.getFrameMetrics()
Log.d("NirvanaEngine", "Parse: ${metrics.parseMs}ms, Style: ${metrics.styleMs}ms, Layout: ${metrics.layoutMs}ms, Raster: ${metrics.rasterMs}ms, Total: ${metrics.totalMs}ms")

// 7. Cleanup
engine.close()
```

---

## 4. User Interaction & Virtual Keyboard (IME)

`NirvanaView` implements Android's `InputConnection` protocols:

- **Touch Hit-Testing:** Tapping on an `<input>` element automatically focuses the element, positions the cursor, and summons the Android soft keyboard (`InputMethodManager`).
- **Text Entry:** Soft keyboard key strokes call `engine.insertText(text)` directly updating the native engine's text buffers and re-rendering the field.
- **Backspace / Deletion:** Calling `engine.deleteBackward()` removes characters.
- **Form Blurring:** Tapping outside an input field automatically blurs focus and dismisses the keyboard.

---

## 5. Web Storage, Cookies & Privacy Controls

Nirvana provides built-in sovereign privacy controls:

```kotlin
// Clear all cookies stored in the RFC 6265 CookieJar
engine.clearCookies()

// Clear all origin-isolated localStorage and sessionStorage
engine.clearStorage()

// Clear in-memory decoded image bitmap buffers
engine.clearImageCache()
```

---

## 6. Memory Lifecycle Management

Android devices enforce strict memory limits. Follow these guidelines:

1. **Activity `onTrimMemory`:**
   ```kotlin
   override fun onTrimMemory(level: Int) {
       super.onTrimMemory(level)
       if (level >= ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW) {
           browserView.engine.clearImageCache()
       }
   }
   ```
2. **Activity `onDestroy`:**
   Always ensure `engine.close()` is called. `NirvanaEngine` implements `AutoCloseable` and frees the native heap pointer safely via `Java_com_nirvana_browser_engine_NirvanaEngine_nativeDestroy`.
