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
 * Automatically synchronizes surface size changes and renders directly onto the hardware canvas.
 */
class NirvanaView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : SurfaceView(context, attrs, defStyleAttr), SurfaceHolder.Callback {

    private var engine: NirvanaEngine? = null
    private var pixelBitmap: Bitmap? = null
    private var pixelBuffer: ByteBuffer? = null
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
        val w = width.coerceAtLeast(1)
        val h = height.coerceAtLeast(1)
        val eng = NirvanaEngine(w, h)
        engine = eng

        eng.loadHtml(currentHtml, currentCss)
        eng.render()
        allocateBitmap(w, h)
        drawFrame()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        val w = width.coerceAtLeast(1)
        val h = height.coerceAtLeast(1)

        engine?.let { eng ->
            eng.resize(w, h)
            eng.render()
            allocateBitmap(w, h)
            drawFrame()
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        engine?.close()
        engine = null
        pixelBitmap?.recycle()
        pixelBitmap = null
        pixelBuffer = null
    }

    private fun allocateBitmap(width: Int, height: Int) {
        if (pixelBitmap?.width != width || pixelBitmap?.height != height) {
            pixelBitmap?.recycle()
            pixelBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            pixelBuffer = ByteBuffer.allocate(width * height * 4)
        }
    }

    private fun drawFrame() {
        val eng = engine ?: return
        val bmp = pixelBitmap ?: return
        val buf = pixelBuffer ?: return

        val pixels = eng.getPixels()
        if (pixels.isEmpty()) return

        buf.rewind()
        buf.put(pixels)
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
