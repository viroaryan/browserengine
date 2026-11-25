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
 * High-resolution performance metrics for individual rendering lifecycle stages.
 */
data class FrameMetrics(
    val parseMs: Float,
    val styleMs: Float,
    val layoutMs: Float,
    val rasterMs: Float,
    val totalMs: Float
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
        @Synchronized get() {
            checkOpen()
            return nativeGetWidth(nativePtr)
        }

    /** Viewport height in pixels. */
    val height: Int
        @Synchronized get() {
            checkOpen()
            return nativeGetHeight(nativePtr)
        }

    /**
     * Updates the scale factor (e.g. mobile DPR / display density).
     */
    @Synchronized
    fun setScale(scale: Float) {
        checkOpen()
        nativeSetScale(nativePtr, scale)
    }

    /** Viewport scale factor (density ratio). */
    val scale: Float
        @Synchronized get() {
            checkOpen()
            return nativeGetScale(nativePtr)
        }

    /**
     * Loads and parses an HTML document with optional CSS rules.
     */
    @Synchronized
    fun loadHtml(html: String, css: String = "") {
        checkOpen()
        nativeLoadHtml(nativePtr, html, css)
    }

    /**
     * Navigates to a given URL string over the network or protocol handler.
     * Fetches document, resolves external linked stylesheets, and executes layout rendering.
     * Returns true on success, false on failure.
     */
    @Synchronized
    fun navigate(url: String): Boolean {
        checkOpen()
        return nativeNavigate(nativePtr, url)
    }

    /**
     * Captures a lock-free snapshot pointer for off-thread downloading. Must be consumed by [executeNavPrefetch].
     */
    @Synchronized
    fun beginNavPrefetch(url: String): Long {
        if (isClosed.get() || nativePtr == 0L) return 0L
        return nativeBeginNavPrefetch(nativePtr, url)
    }

    /**
     * Performs HTTP fetching and concurrent subresource prefetching without locking the UI render engine.
     */
    fun executeNavPrefetch(snapshotPtr: Long, url: String): Long {
        if (snapshotPtr == 0L) return 0L
        return nativeExecuteNavPrefetch(snapshotPtr, url)
    }

    /**
     * Commits (or frees when [apply] is false) a prefetched navigation result into the engine.
     */
    @Synchronized
    fun commitNavResult(resultPtr: Long, apply: Boolean): Boolean {
        if (resultPtr == 0L) return false
        val targetPtr = if (apply && !isClosed.get() && nativePtr != 0L) nativePtr else 0L
        return nativeCommitNavResult(targetPtr, resultPtr, apply && targetPtr != 0L)
    }

    /** The canonical URL of the currently loaded document, or null if none. */
    val currentUrl: String?
        @Synchronized get() {
            checkOpen()
            return nativeGetCurrentUrl(nativePtr)
        }

    /** The last navigation error string, or null if last navigation succeeded. */
    val lastError: String?
        @Synchronized get() {
            checkOpen()
            return nativeGetLastError(nativePtr)
        }

    /**
     * Resizes the engine's viewport dimensions.
     */
    @Synchronized
    fun resize(newWidth: Int, newHeight: Int) {
        checkOpen()
        nativeResize(nativePtr, newWidth.coerceAtLeast(1), newHeight.coerceAtLeast(1))
    }

    /**
     * Executes the layout and rasterization pipeline.
     */
    @Synchronized
    fun render() {
        checkOpen()
        nativeRender(nativePtr)
    }

    /**
     * Retrieves the rasterized 32-bit RGBA pixel buffer.
     * Dimensions are [width] x [height] x 4 bytes.
     */
    @Synchronized
    fun getPixels(): ByteArray {
        checkOpen()
        return nativeGetPixels(nativePtr)
    }

    /**
     * Copies the rasterized pixels directly into a pre-allocated direct [java.nio.ByteBuffer].
     * Zero-allocation transfer mechanism eliminating GC churn.
     * Returns true on success, false if the buffer capacity is insufficient or native copy fails.
     */
    @Synchronized
    fun copyPixelsToBuffer(directBuffer: java.nio.ByteBuffer): Boolean {
        checkOpen()
        require(directBuffer.isDirect) { "Buffer must be allocated via ByteBuffer.allocateDirect()" }
        return nativeCopyPixelsToBuffer(nativePtr, directBuffer) == 0
    }

    /** Horizontal scroll offset in pixels. */
    val scrollX: Float
        @Synchronized get() {
            checkOpen()
            return nativeGetScrollX(nativePtr)
        }

    /** Vertical scroll offset in pixels. */
    val scrollY: Float
        @Synchronized get() {
            checkOpen()
            return nativeGetScrollY(nativePtr)
        }

    /** Maximum vertical scroll offset in pixels. */
    val maxScrollY: Float
        @Synchronized get() {
            checkOpen()
            return nativeGetMaxScrollY(nativePtr)
        }

    /**
     * Scrolls the viewport by the given delta (dx, dy).
     */
    @Synchronized
    fun scrollBy(dx: Float, dy: Float) {
        checkOpen()
        nativeScrollBy(nativePtr, dx, dy)
    }

    /**
     * Sets the viewport scroll offset to the specified (x, y) coordinates.
     */
    @Synchronized
    fun scrollTo(x: Float, y: Float) {
        checkOpen()
        nativeScrollTo(nativePtr, x, y)
    }

    /**
     * Performs a hit-test at the given screen coordinates (x, y).
     * Returns a [HitTarget] if an element or link was tapped, or null otherwise.
     */
    @Synchronized
    fun hitTest(screenX: Float, screenY: Float): HitTarget? {
        checkOpen()
        val json = nativeHitTest(nativePtr, screenX, screenY) ?: return null
        if (json.isEmpty()) return null
        return parseHitTargetJson(json)
    }

    /**
     * Handles a click or tap at (screenX, screenY).
     * If an interactive form control (<input> or <textarea>) is clicked,
     * focuses it, positions the caret, and returns the HitTarget.
     * If a non-input is clicked, blurs.
     */
    @Synchronized
    fun handleClick(screenX: Float, screenY: Float): HitTarget? {
        checkOpen()
        val json = nativeHandleClick(nativePtr, screenX, screenY) ?: return null
        if (json.isEmpty()) return null
        return parseHitTargetJson(json)
    }

    /** Returns true if an <input> or <textarea> element currently has focus. */
    val isInputFocused: Boolean
        @Synchronized get() {
            checkOpen()
            return nativeIsInputFocused(nativePtr)
        }

    /** Inserts text at the current caret position of the focused form control. */
    @Synchronized
    fun insertText(text: String) {
        checkOpen()
        nativeInsertText(nativePtr, text)
    }

    /** Deletes the character before the caret in the focused form control. */
    @Synchronized
    fun deleteBackward() {
        checkOpen()
        nativeDeleteBackward(nativePtr)
    }

    /** Text of the currently focused element, or null if none. */
    val focusedText: String?
        @Synchronized get() {
            checkOpen()
            return nativeGetFocusedText(nativePtr)
        }

    /** Screen bounds [x, y, width, height] of the currently focused element, or null if none. */
    val focusedBounds: FloatArray?
        @Synchronized get() {
            checkOpen()
            return nativeGetFocusedBounds(nativePtr)
        }

    /** Removes focus from any focused element. */
    @Synchronized
    fun blur() {
        checkOpen()
        nativeBlur(nativePtr)
    }

    /**
     * Evaluates a JavaScript script string in the active JS runtime context.
     * Returns the evaluation result string, or null on error / empty.
     */
    @Synchronized
    fun evaluateJs(script: String): String? {
        checkOpen()
        return nativeEvaluateJs(nativePtr, script)
    }

    /**
     * Steps the active JS event loop timers by [deltaMs] milliseconds and fires expired timers.
     * Returns the number of fired timers.
     */
    @Synchronized
    fun stepTimers(deltaMs: Long): Int {
        checkOpen()
        return nativeStepTimers(nativePtr, deltaMs)
    }

    /**
     * Clears all cookies stored in the sovereign CookieJar.
     */
    @Synchronized
    fun clearCookies() {
        checkOpen()
        nativeClearCookies(nativePtr)
    }

    /**
     * Clears all local and session Web Storage across all origins.
     */
    @Synchronized
    fun clearStorage() {
        checkOpen()
        nativeClearStorage(nativePtr)
    }

    /**
     * Clears the in-memory decoded image bitmap cache.
     */
    @Synchronized
    fun clearImageCache() {
        checkOpen()
        nativeClearImageCache(nativePtr)
    }

    /**
     * Retrieves high-resolution timing metrics for the most recent render frame.
     */
    @Synchronized
    fun getFrameMetrics(): FrameMetrics {
        checkOpen()
        val metrics = nativeGetFrameMetrics(nativePtr)
        return if (metrics != null && metrics.size >= 5) {
            FrameMetrics(
                parseMs = metrics[0],
                styleMs = metrics[1],
                layoutMs = metrics[2],
                rasterMs = metrics[3],
                totalMs = metrics[4]
            )
        } else {
            FrameMetrics(0f, 0f, 0f, 0f, 0f)
        }
    }

    /**
     * Retrieves the total count of blocked ad and tracker network requests.
     */
    @Synchronized
    fun getBlockedRequestsCount(): Long {
        checkOpen()
        return nativeGetBlockedCount(nativePtr)
    }

    /**
     * Enables or disables sovereign ad blocking.
     */
    @Synchronized
    fun setAdBlockEnabled(enabled: Boolean) {
        checkOpen()
        nativeSetAdBlockEnabled(nativePtr, enabled)
    }

    /**
     * Returns true if sovereign ad blocking is currently active.
     */
    @Synchronized
    fun isAdBlockEnabled(): Boolean {
        checkOpen()
        return nativeIsAdBlockEnabled(nativePtr)
    }

    /**
     * Parses and registers a custom AdBlock Plus / uBlock filter rule string.
     */
    @Synchronized
    fun addAdBlockRule(rule: String) {
        checkOpen()
        nativeAddAdBlockRule(nativePtr, rule)
    }

    /**
     * Resets the blocked requests counter to zero.
     */
    @Synchronized
    fun resetBlockedCount() {
        checkOpen()
        nativeResetBlockedCount(nativePtr)
    }

    /** Title extracted from the current document's `<title>` tag, or null if absent. */
    val pageTitle: String?
        @Synchronized get() {
            checkOpen()
            return nativeGetPageTitle(nativePtr)
        }

    /**
     * Finds text matches across the active layout tree, scrolls to `matchIndex`, and returns total match count.
     */
    @Synchronized
    fun findInPage(query: String, matchIndex: Int): Int {
        checkOpen()
        return nativeFindInPage(nativePtr, query, matchIndex)
    }

    /**
     * Extracts clean Reader View HTML from the active document DOM, or null if unsuitable.
     */
    @Synchronized
    fun extractReaderHtml(): String? {
        checkOpen()
        return nativeExtractReaderHtml(nativePtr)
    }

    /**
     * Returns extracted plain text snippet from the current DOM.
     */
    @Synchronized
    fun getPageTextSnippet(maxLen: Int = 2048): String {
        checkOpen()
        return nativeGetPageTextSnippet(nativePtr, maxLen) ?: ""
    }

    /**
     * Applies runtime engine feature & privacy preferences.
     */
    @Synchronized
    fun configure(
        desktopMode: Boolean,
        httpsOnly: Boolean,
        jsEnabled: Boolean,
        forceDark: Boolean,
        doNotTrack: Boolean
    ) {
        checkOpen()
        nativeConfigure(nativePtr, desktopMode, httpsOnly, jsEnabled, forceDark, doNotTrack)
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

    @Synchronized
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
    private external fun nativeSetScale(enginePtr: Long, scale: Float)
    private external fun nativeGetScale(enginePtr: Long): Float
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
    private external fun nativeHandleClick(enginePtr: Long, x: Float, y: Float): String?
    private external fun nativeIsInputFocused(enginePtr: Long): Boolean
    private external fun nativeInsertText(enginePtr: Long, text: String)
    private external fun nativeDeleteBackward(enginePtr: Long)
    private external fun nativeGetFocusedText(enginePtr: Long): String?
    private external fun nativeGetFocusedBounds(enginePtr: Long): FloatArray?
    private external fun nativeBlur(enginePtr: Long)
    private external fun nativeNavigate(enginePtr: Long, url: String): Boolean
    private external fun nativeGetCurrentUrl(enginePtr: Long): String?
    private external fun nativeGetLastError(enginePtr: Long): String?
    private external fun nativeEvaluateJs(enginePtr: Long, script: String): String?
    private external fun nativeStepTimers(enginePtr: Long, deltaMs: Long): Int
    private external fun nativeClearCookies(enginePtr: Long)
    private external fun nativeClearStorage(enginePtr: Long)
    private external fun nativeClearImageCache(enginePtr: Long)
    private external fun nativeGetFrameMetrics(enginePtr: Long): FloatArray?
    private external fun nativeGetBlockedCount(enginePtr: Long): Long
    private external fun nativeSetAdBlockEnabled(enginePtr: Long, enabled: Boolean)
    private external fun nativeIsAdBlockEnabled(enginePtr: Long): Boolean
    private external fun nativeAddAdBlockRule(enginePtr: Long, rule: String)
    private external fun nativeResetBlockedCount(enginePtr: Long)
    private external fun nativeGetPageTitle(enginePtr: Long): String?
    private external fun nativeFindInPage(enginePtr: Long, query: String, matchIndex: Int): Int
    private external fun nativeExtractReaderHtml(enginePtr: Long): String?
    private external fun nativeGetPageTextSnippet(enginePtr: Long, maxLen: Int): String?
    private external fun nativeConfigure(
        enginePtr: Long,
        desktopMode: Boolean,
        httpsOnly: Boolean,
        jsEnabled: Boolean,
        forceDark: Boolean,
        doNotTrack: Boolean
    )
    private external fun nativeBeginNavPrefetch(enginePtr: Long, url: String): Long
    private external fun nativeExecuteNavPrefetch(snapshotPtr: Long, url: String): Long
    private external fun nativeCommitNavResult(enginePtr: Long, resultPtr: Long, apply: Boolean): Boolean

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
