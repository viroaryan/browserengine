package com.nirvana.browser.engine

import java.io.Closeable
import java.util.concurrent.atomic.AtomicBoolean

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
