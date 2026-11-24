package com.nirvana.browser.engine

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Rect
import android.util.AttributeSet
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.OverScroller
import java.nio.ByteBuffer

/**
 * Custom Android SurfaceView hosting the Nirvana Browser Engine.
 * Automatically synchronizes surface size changes and renders directly onto the surface canvas
 * using zero-allocation direct byte buffers.
 * Supports smooth touch pan dragging, kinetic OverScroller fling inertia, and link navigation.
 */
class NirvanaView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : SurfaceView(context, attrs, defStyleAttr), SurfaceHolder.Callback {

    /**
     * Listener callback invoked when the user taps on an anchor link.
     */
    fun interface OnLinkClickListener {
        fun onLinkClick(url: String)
    }

    private val engineLock = Any()
    private var engine: NirvanaEngine? = null
    private var pixelBitmap: Bitmap? = null
    private var directPixelBuffer: ByteBuffer? = null
    private var currentHtml: String = "<html><body><h1>Nirvana Browser</h1></body></html>"
    private var currentCss: String = ""
    private var onLinkClickListener: OnLinkClickListener? = null
    private val networkExecutor = java.util.concurrent.Executors.newSingleThreadExecutor()
    private var activeNavigationFuture: java.util.concurrent.Future<*>? = null

    private val scroller: OverScroller = OverScroller(context)
    private val gestureDetector: GestureDetector

    private val flingRunnable = object : Runnable {
        override fun run() {
            if (scroller.computeScrollOffset()) {
                synchronized(engineLock) {
                    val eng = engine ?: return
                    eng.scrollTo(scroller.currX.toFloat(), scroller.currY.toFloat())
                    drawFrameLocked(eng)
                }
                postOnAnimation(this)
            }
        }
    }

    init {
        holder.addCallback(this)

        val gestureListener = object : GestureDetector.SimpleOnGestureListener() {
            override fun onDown(e: MotionEvent): Boolean {
                if (!scroller.isFinished) {
                    scroller.abortAnimation()
                    removeCallbacks(flingRunnable)
                }
                return true
            }

            override fun onScroll(
                e1: MotionEvent?,
                e2: MotionEvent,
                distanceX: Float,
                distanceY: Float
            ): Boolean {
                synchronized(engineLock) {
                    val eng = engine ?: return false
                    eng.scrollBy(distanceX, distanceY)
                    drawFrameLocked(eng)
                }
                return true
            }

            override fun onFling(
                e1: MotionEvent?,
                e2: MotionEvent,
                velocityX: Float,
                velocityY: Float
            ): Boolean {
                val (startX, startY, maxY) = synchronized(engineLock) {
                    val eng = engine ?: return false
                    Triple(eng.scrollX.toInt(), eng.scrollY.toInt(), eng.maxScrollY.toInt())
                }

                scroller.abortAnimation()
                scroller.fling(
                    startX, startY,
                    (-velocityX).toInt(), (-velocityY).toInt(),
                    0, 0,
                    0, maxY.coerceAtLeast(0)
                )
                postOnAnimation(flingRunnable)
                return true
            }

            override fun onSingleTapUp(e: MotionEvent): Boolean {
                val hit = synchronized(engineLock) {
                    engine?.hitTest(e.x, e.y)
                }
                if (hit != null && !hit.href.isNullOrEmpty()) {
                    val href = hit.href
                    if (onLinkClickListener != null) {
                        onLinkClickListener?.onLinkClick(href)
                    } else {
                        loadUrl(href)
                    }
                    return true
                }
                return super.onSingleTapUp(e)
            }
        }

        gestureDetector = GestureDetector(context, gestureListener)
    }

    /**
     * Registers a listener for user anchor link taps.
     */
    fun setOnLinkClickListener(listener: OnLinkClickListener?) {
        this.onLinkClickListener = listener
    }

    /**
     * Loads HTML content and triggers a re-render.
     */
    fun loadHtml(html: String, css: String = "") {
        currentHtml = html
        currentCss = css
        synchronized(engineLock) {
            engine?.let { eng ->
                eng.loadHtml(html, css)
                eng.render()
                drawFrameLocked(eng)
            }
        }
    }

    /**
     * Navigates to a web URL asynchronously on a background worker thread.
     * On network completion, dispatches frame redraw back to the UI thread.
     */
    fun loadUrl(url: String) {
        synchronized(engineLock) {
            if (engine == null) {
                val eng = NirvanaEngine(width.coerceAtLeast(1), height.coerceAtLeast(1))
                engine = eng
                allocateBuffers(eng.width, eng.height)
            }
        }

        activeNavigationFuture?.cancel(true)
        activeNavigationFuture = networkExecutor.submit {
            val eng = synchronized(engineLock) { engine } ?: return@submit
            if (Thread.currentThread().isInterrupted) return@submit

            val success = try {
                eng.navigate(url)
            } catch (e: Exception) {
                false
            }

            if (success && !Thread.currentThread().isInterrupted) {
                post {
                    synchronized(engineLock) {
                        val currentEng = engine ?: return@post
                        drawFrameLocked(currentEng)
                    }
                }
            }
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val handled = gestureDetector.onTouchEvent(event)
        return handled || super.onTouchEvent(event)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        // Surface is ready; actual dimensions arrive in surfaceChanged immediately.
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        val w = width.coerceAtLeast(1)
        val h = height.coerceAtLeast(1)

        synchronized(engineLock) {
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
            drawFrameLocked(eng)
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        removeCallbacks(flingRunnable)
        scroller.abortAnimation()
        activeNavigationFuture?.cancel(true)
        activeNavigationFuture = null

        synchronized(engineLock) {
            engine?.close()
            engine = null
            pixelBitmap?.recycle()
            pixelBitmap = null
            directPixelBuffer = null
        }
    }

    private fun allocateBuffers(width: Int, height: Int) {
        if (pixelBitmap?.width != width || pixelBitmap?.height != height) {
            pixelBitmap?.recycle()
            pixelBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            directPixelBuffer = ByteBuffer.allocateDirect(width * height * 4)
        }
    }

    private fun drawFrameLocked(eng: NirvanaEngine) {
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

    private fun drawFrame() {
        synchronized(engineLock) {
            val eng = engine ?: return
            drawFrameLocked(eng)
        }
    }
}
