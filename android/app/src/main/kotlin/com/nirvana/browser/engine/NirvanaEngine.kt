package com.nirvana.browser.engine

import java.io.Closeable
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Interactive target hit by a touch or pointer event.
 */
data class HitTarget(
    val tag: String,
    val href: String?,
    val text: String?
)

/**
 * Idiomatic Kotlin wrapper around the high-performance Rust Nirvana Browser Engine.
 * Manages native lifecycle, HTML/CSS loading, layout execution, and pixel extraction.
 */
class NirvanaEngine(
    initialWidth: Int = 1080,
    initialHeight: Int = 1920
) : Closeable, AutoCloseable {

    private var nativePtr: Long = 0
    private val isClosed = AtomicBoolean(false)

    init {
        nativePtr = nativeCreate(initialWidth.coerceAtLeast(1), initialHeight.coerceAtLeast(1))
        check(nativePtr != 0L) { "Failed to initialize native NirvanaEngine instance" }
    }

    /** Viewport width in pixels. */
    val width: Int
        get() {
            checkOpen()
            return nativeGetWidth(nativePtr)
        }

    /** Viewport height in pixels. */
    val height: Int
        get() {
            checkOpen()
            return nativeGetHeight(nativePtr)
        }

    /**
     * Loads and parses an HTML document with optional CSS rules.
     */
    fun loadHtml(html: String, css: String = "") {
        checkOpen()
        nativeLoadHtml(nativePtr, html, css)
    }

    /**
     * Navigates to a given URL string over the network or protocol handler.
     * Fetches document, resolves external linked stylesheets, and executes layout rendering.
     * Returns true on success, false on failure.
     */
    fun navigate(url: String): Boolean {
        checkOpen()
        return nativeNavigate(nativePtr, url)
    }

    /** The canonical URL of the currently loaded document, or null if none. */
    val currentUrl: String?
        get() {
            checkOpen()
            return nativeGetCurrentUrl(nativePtr)
        }

    /**
     * Resizes the engine's viewport dimensions.
     */
    fun resize(newWidth: Int, newHeight: Int) {
        checkOpen()
        nativeResize(nativePtr, newWidth.coerceAtLeast(1), newHeight.coerceAtLeast(1))
    }

    /**
     * Executes the layout and rasterization pipeline.
     */
    fun render() {
        checkOpen()
        nativeRender(nativePtr)
    }

    /**
     * Retrieves the rasterized 32-bit RGBA pixel buffer.
     * Dimensions are [width] x [height] x 4 bytes.
     */
    fun getPixels(): ByteArray {
        checkOpen()
        return nativeGetPixels(nativePtr)
    }

    /**
     * Copies the rasterized pixels directly into a pre-allocated direct [java.nio.ByteBuffer].
     * Zero-allocation transfer mechanism eliminating GC churn.
     * Returns true on success, false if the buffer capacity is insufficient or native copy fails.
     */
    fun copyPixelsToBuffer(directBuffer: java.nio.ByteBuffer): Boolean {
        checkOpen()
        require(directBuffer.isDirect) { "Buffer must be allocated via ByteBuffer.allocateDirect()" }
        return nativeCopyPixelsToBuffer(nativePtr, directBuffer) == 0
    }

    /** Horizontal scroll offset in pixels. */
    val scrollX: Float
        get() {
            checkOpen()
            return nativeGetScrollX(nativePtr)
        }

    /** Vertical scroll offset in pixels. */
    val scrollY: Float
        get() {
            checkOpen()
            return nativeGetScrollY(nativePtr)
        }

    /** Maximum vertical scroll offset in pixels. */
    val maxScrollY: Float
        get() {
            checkOpen()
            return nativeGetMaxScrollY(nativePtr)
        }

    /**
     * Scrolls the viewport by the given delta (dx, dy).
     */
    fun scrollBy(dx: Float, dy: Float) {
        checkOpen()
        nativeScrollBy(nativePtr, dx, dy)
    }

    /**
     * Sets the viewport scroll offset to the specified (x, y) coordinates.
     */
    fun scrollTo(x: Float, y: Float) {
        checkOpen()
        nativeScrollTo(nativePtr, x, y)
    }

    /**
     * Performs a hit-test at the given screen coordinates (x, y).
     * Returns a [HitTarget] if an element or link was tapped, or null otherwise.
     */
    fun hitTest(screenX: Float, screenY: Float): HitTarget? {
        checkOpen()
        val json = nativeHitTest(nativePtr, screenX, screenY) ?: return null
        if (json.isEmpty()) return null
        return parseHitTargetJson(json)
    }

    private fun parseHitTargetJson(json: String): HitTarget? {
        val tagMatch = Regex("\"tag\"\\s*:\\s*\"([^\"]*)\"").find(json) ?: return null
        val tag = tagMatch.groupValues[1]

        val hrefMatch = Regex("\"href\"\\s*:\\s*(?:\"([^\"]*)\"|null)").find(json)
        val href = hrefMatch?.groupValues?.getOrNull(1)?.takeIf { it.isNotEmpty() }

        val textMatch = Regex("\"text\"\\s*:\\s*(?:\"([^\"]*)\"|null)").find(json)
        val text = textMatch?.groupValues?.getOrNull(1)?.takeIf { it.isNotEmpty() }

        return HitTarget(tag, href, text)
    }

    override fun close() {
        if (isClosed.compareAndSet(false, true)) {
            val ptr = nativePtr
            nativePtr = 0
            if (ptr != 0L) {
                nativeDestroy(ptr)
            }
        }
    }

    private fun checkOpen() {
        check(!isClosed.get() && nativePtr != 0L) { "NirvanaEngine has already been closed" }
    }

    // --- Native JNI External Declarations ---

    private external fun nativeCreate(width: Int, height: Int): Long
    private external fun nativeDestroy(enginePtr: Long)
    private external fun nativeLoadHtml(enginePtr: Long, html: String, css: String)
    private external fun nativeResize(enginePtr: Long, width: Int, height: Int)
    private external fun nativeRender(enginePtr: Long)
    private external fun nativeGetWidth(enginePtr: Long): Int
    private external fun nativeGetHeight(enginePtr: Long): Int
    private external fun nativeGetPixels(enginePtr: Long): ByteArray
    private external fun nativeCopyPixelsToBuffer(enginePtr: Long, buffer: java.nio.ByteBuffer): Int
    private external fun nativeScrollBy(enginePtr: Long, dx: Float, dy: Float)
    private external fun nativeScrollTo(enginePtr: Long, x: Float, y: Float)
    private external fun nativeGetScrollX(enginePtr: Long): Float
    private external fun nativeGetScrollY(enginePtr: Long): Float
    private external fun nativeGetMaxScrollY(enginePtr: Long): Float
    private external fun nativeHitTest(enginePtr: Long, x: Float, y: Float): String?
    private external fun nativeNavigate(enginePtr: Long, url: String): Boolean
    private external fun nativeGetCurrentUrl(enginePtr: Long): String?

    companion object {
        init {
            try {
                System.loadLibrary("nirvana_engine")
            } catch (e: UnsatisfiedLinkError) {
                System.err.println("Warning: libnirvana_engine.so not loaded: ${e.message}")
            }
        }
    }
}
