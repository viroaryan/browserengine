package com.nirvana.browser.engine

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Rect
import android.util.AttributeSet
import android.view.SurfaceHolder
import android.view.SurfaceView
import java.nio.ByteBuffer

/**
 * Custom Android SurfaceView hosting the Nirvana Browser Engine.
 * Automatically synchronizes surface size changes and renders directly onto the surface canvas
 * using zero-allocation direct byte buffers.
 */
class NirvanaView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : SurfaceView(context, attrs, defStyleAttr), SurfaceHolder.Callback {

    private var engine: NirvanaEngine? = null
    private var pixelBitmap: Bitmap? = null
    private var directPixelBuffer: ByteBuffer? = null
    private var currentHtml: String = "<html><body><h1>Nirvana Browser</h1></body></html>"
    private var currentCss: String = ""

    init {
        holder.addCallback(this)
    }

    /**
     * Loads HTML content and triggers a re-render.
     */
    fun loadHtml(html: String, css: String = "") {
        currentHtml = html
        currentCss = css
        engine?.let { eng ->
            eng.loadHtml(html, css)
            eng.render()
            drawFrame()
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        // Surface is ready; actual dimensions are guaranteed to arrive in surfaceChanged immediately.
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        val w = width.coerceAtLeast(1)
        val h = height.coerceAtLeast(1)

        val eng = engine ?: NirvanaEngine(w, h).also {
            it.loadHtml(currentHtml, currentCss)
            engine = it
        }

        if (eng.width != w || eng.height != h) {
            eng.resize(w, h)
        } else {
            eng.render()
        }

        allocateBuffers(w, h)
        drawFrame()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        engine?.close()
        engine = null
        pixelBitmap?.recycle()
        pixelBitmap = null
        directPixelBuffer = null
    }

    private fun allocateBuffers(width: Int, height: Int) {
        if (pixelBitmap?.width != width || pixelBitmap?.height != height) {
            pixelBitmap?.recycle()
            pixelBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            // Allocate off-heap direct byte buffer to eliminate GC allocations during frame copies
            directPixelBuffer = ByteBuffer.allocateDirect(width * height * 4)
        }
    }

    private fun drawFrame() {
        val eng = engine ?: return
        val bmp = pixelBitmap ?: return
        val buf = directPixelBuffer ?: return

        buf.rewind()
        val success = eng.copyPixelsToBuffer(buf)
        if (!success) return

        buf.rewind()
        bmp.copyPixelsFromBuffer(buf)

        var canvas: Canvas? = null
        try {
            canvas = holder.lockCanvas()
            if (canvas != null) {
                val srcRect = Rect(0, 0, eng.width, eng.height)
                val dstRect = Rect(0, 0, width, height)
                canvas.drawBitmap(bmp, srcRect, dstRect, null)
            }
        } finally {
            if (canvas != null) {
                holder.unlockCanvasAndPost(canvas)
            }
        }
    }
}
