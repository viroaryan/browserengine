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
import java.net.URL
import java.nio.ByteBuffer

/**
 * Custom Android SurfaceView hosting the Sovereign Rust Nirvana Browser Engine.
 * Supports high-DPI scaling, logical-unit smooth panning & fling inertia, IME text input, and link navigation.
 */
class NirvanaView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : SurfaceView(context, attrs, defStyleAttr), SurfaceHolder.Callback {

    fun interface OnLinkClickListener {
        fun onLinkClick(url: String)
    }

    private val engineLock = Any()
    private var _engine: NirvanaEngine? = null
    @Volatile
    private var hasLoadedContent: Boolean = false

    /** Multiplier on display density (controlled by Settings -> Text & Display Zoom). */
    var userZoomFactor: Float = 1.0f
        set(value) {
            field = value.coerceIn(0.5f, 2.5f)
            synchronized(engineLock) {
                val eng = _engine ?: return
                eng.setScale(resources.displayMetrics.density * field)
                drawFrameLocked(eng)
            }
        }

    /** Exposes the active or initialized NirvanaEngine instance. */
    val engine: NirvanaEngine
        get() = synchronized(engineLock) {
            _engine ?: NirvanaEngine(width.coerceAtLeast(1080), height.coerceAtLeast(1920)).also {
                it.setScale(resources.displayMetrics.density * userZoomFactor)
                _engine = it
            }
        }

    private var pixelBitmap: Bitmap? = null
    private var directPixelBuffer: ByteBuffer? = null
    private var currentHtml: String = "<html><body><h1>Nirvana Browser</h1></body></html>"
    private var currentCss: String = ""

    var onLinkClickListener: ((String) -> Unit)? = null
    private val networkExecutor = java.util.concurrent.Executors.newSingleThreadExecutor()
    private var activeNavigationFuture: java.util.concurrent.Future<*>? = null

    private val scroller: OverScroller = OverScroller(context)
    private val gestureDetector: GestureDetector

    private val flingRunnable = object : Runnable {
        override fun run() {
            if (scroller.computeScrollOffset()) {
                synchronized(engineLock) {
                    val eng = _engine ?: return
                    val s = eng.scale.coerceAtLeast(1f)
                    eng.scrollTo(scroller.currX / s, scroller.currY / s)
                    drawFrameLocked(eng)
                }
                postOnAnimation(this)
            }
        }
    }

    init {
        holder.addCallback(this)
        isFocusable = true
        isFocusableInTouchMode = true

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
                    val eng = _engine ?: return false
                    val s = eng.scale.coerceAtLeast(1f)
                    eng.scrollBy(distanceX / s, distanceY / s)
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
                    val eng = _engine ?: return false
                    val s = eng.scale.coerceAtLeast(1f)
                    Triple((eng.scrollX * s).toInt(), (eng.scrollY * s).toInt(), (eng.maxScrollY * s).toInt())
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
                val wasFocused = synchronized(engineLock) { _engine?.isInputFocused == true }
                val (hit, isInputFocused, baseHref) = synchronized(engineLock) {
                    val eng = _engine ?: return false
                    val target = eng.handleClick(e.x, e.y)
                    val focused = eng.isInputFocused
                    if (wasFocused || focused) {
                        drawFrameLocked(eng)
                    }
                    Triple(target, focused, eng.currentUrl)
                }

                val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as? android.view.inputmethod.InputMethodManager
                if (isInputFocused) {
                    requestFocus()
                    imm?.showSoftInput(this@NirvanaView, android.view.inputmethod.InputMethodManager.SHOW_IMPLICIT)
                } else {
                    requestFocus()
                    imm?.hideSoftInputFromWindow(windowToken, 0)
                }

                if (hit != null && !hit.href.isNullOrEmpty()) {
                    val resolved = resolveUrl(baseHref, hit.href)
                    val listener = onLinkClickListener
                    if (listener != null) {
                        listener.invoke(resolved)
                    } else {
                        loadUrl(resolved)
                    }
                    return true
                }
                return super.onSingleTapUp(e)
            }
        }

        gestureDetector = GestureDetector(context, gestureListener)
    }

    private fun resolveUrl(baseUrl: String?, rawHref: String): String {
        val cleaned = rawHref.trim()
        if (cleaned.startsWith("http://") || cleaned.startsWith("https://") || cleaned.startsWith("data:")) {
            return cleaned
        }
        if (cleaned.startsWith("//")) {
            return "https:$cleaned"
        }
        return try {
            if (!baseUrl.isNullOrBlank()) {
                URL(URL(baseUrl), cleaned).toString()
            } else {
                cleaned
            }
        } catch (_: Exception) {
            cleaned
        }
    }

    override fun onCreateInputConnection(outAttrs: android.view.inputmethod.EditorInfo): android.view.inputmethod.InputConnection? {
        outAttrs.inputType = android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_VARIATION_NORMAL
        outAttrs.imeOptions = android.view.inputmethod.EditorInfo.IME_ACTION_DONE

        return object : android.view.inputmethod.BaseInputConnection(this, true) {
            override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
                if (!text.isNullOrEmpty()) {
                    synchronized(engineLock) {
                        val eng = _engine ?: return false
                        eng.insertText(text.toString())
                        drawFrameLocked(eng)
                    }
                }
                return true
            }

            override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
                if (beforeLength > 0) {
                    synchronized(engineLock) {
                        val eng = _engine ?: return false
                        repeat(beforeLength) {
                            eng.deleteBackward()
                        }
                        drawFrameLocked(eng)
                    }
                }
                return true
            }

            override fun performEditorAction(editorAction: Int): Boolean {
                synchronized(engineLock) {
                    val eng = _engine ?: return false
                    eng.blur()
                    drawFrameLocked(eng)
                }
                val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as? android.view.inputmethod.InputMethodManager
                imm?.hideSoftInputFromWindow(windowToken, 0)
                return true
            }

            override fun sendKeyEvent(event: android.view.KeyEvent): Boolean {
                if (event.action == android.view.KeyEvent.ACTION_DOWN) {
                    if (event.keyCode == android.view.KeyEvent.KEYCODE_DEL) {
                        synchronized(engineLock) {
                            val eng = _engine ?: return false
                            eng.deleteBackward()
                            drawFrameLocked(eng)
                        }
                        return true
                    } else if (event.unicodeChar != 0) {
                        val charStr = event.unicodeChar.toChar().toString()
                        synchronized(engineLock) {
                            val eng = _engine ?: return false
                            eng.insertText(charStr)
                            drawFrameLocked(eng)
                        }
                        return true
                    }
                }
                return super.sendKeyEvent(event)
            }
        }
    }

    fun setOnLinkClickListener(listener: OnLinkClickListener?) {
        this.onLinkClickListener = if (listener != null) { { url: String -> listener.onLinkClick(url) } } else null
    }

    fun requestRender() {
        post {
            synchronized(engineLock) {
                val eng = _engine ?: return@post
                drawFrameLocked(eng)
            }
        }
    }

    fun markContentLoaded() {
        hasLoadedContent = true
    }

    /**
     * Loads HTML content and triggers a re-render at current DPI + user zoom scale.
     */
    fun loadHtml(html: String, css: String = "") {
        currentHtml = html
        currentCss = css
        hasLoadedContent = true
        post {
            synchronized(engineLock) {
                val eng = _engine ?: NirvanaEngine(width.coerceAtLeast(1080), height.coerceAtLeast(1920)).also {
                    _engine = it
                }
                eng.setScale(resources.displayMetrics.density * userZoomFactor)
                eng.loadHtml(html, css)
                eng.render()
                drawFrameLocked(eng)
            }
        }
    }

    fun loadUrl(url: String) {
        hasLoadedContent = true
        val snapPtr = synchronized(engineLock) {
            val eng = _engine ?: NirvanaEngine(width.coerceAtLeast(1080), height.coerceAtLeast(1920)).also {
                it.setScale(resources.displayMetrics.density * userZoomFactor)
                _engine = it
                allocateBuffers(it.width, it.height)
            }
            eng.beginNavPrefetch(url)
        }
        if (snapPtr == 0L) return

        activeNavigationFuture?.cancel(true)
        val helperEngine = synchronized(engineLock) { _engine } ?: return
        activeNavigationFuture = networkExecutor.submit {
            val resPtr = try {
                helperEngine.executeNavPrefetch(snapPtr, url)
            } catch (e: Exception) {
                android.util.Log.e("NirvanaView", "Navigation exception for $url", e)
                0L
            }
            if (resPtr == 0L) return@submit

            val shouldApply = !Thread.currentThread().isInterrupted
            val committed = synchronized(engineLock) {
                val eng = _engine
                if (eng == null) {
                    helperEngine.commitNavResult(resPtr, false)
                    false
                } else {
                    eng.commitNavResult(resPtr, shouldApply)
                }
            }
            if (committed) {
                post {
                    synchronized(engineLock) {
                        val currentEng = _engine ?: return@post
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

    override fun surfaceCreated(holder: SurfaceHolder) {}

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        val w = width.coerceAtLeast(1)
        val h = height.coerceAtLeast(1)

        synchronized(engineLock) {
            val eng = _engine ?: NirvanaEngine(w, h).also {
                _engine = it
            }

            if (eng.width != w || eng.height != h) {
                eng.resize(w, h)
            }

            eng.setScale(resources.displayMetrics.density * userZoomFactor)

            if (!hasLoadedContent && eng.currentUrl.isNullOrEmpty()) {
                eng.loadHtml(currentHtml, currentCss)
                hasLoadedContent = true
            }
            eng.render()

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
            pixelBitmap?.recycle()
            pixelBitmap = null
            directPixelBuffer = null
        }
    }

    /**
     * Releases the underlying native Rust engine instance when the hosting Activity exits.
     */
    fun destroyEngine() {
        removeCallbacks(flingRunnable)
        scroller.abortAnimation()
        activeNavigationFuture?.cancel(true)
        activeNavigationFuture = null

        synchronized(engineLock) {
            _engine?.close()
            _engine = null
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
        val w = eng.width
        val h = eng.height
        if (w <= 0 || h <= 0) return

        allocateBuffers(w, h)
        val bmp = pixelBitmap ?: return
        val buf = directPixelBuffer ?: return

        buf.rewind()
        if (!eng.copyPixelsToBuffer(buf)) return

        buf.rewind()
        bmp.copyPixelsFromBuffer(buf)

        if (!holder.surface.isValid) return

        var canvas: Canvas? = null
        try {
            canvas = holder.lockCanvas()
            if (canvas != null) {
                canvas.drawColor(android.graphics.Color.WHITE)
                val srcRect = Rect(0, 0, w, h)
                val dstRect = Rect(0, 0, width.coerceAtLeast(1), height.coerceAtLeast(1))
                canvas.drawBitmap(bmp, srcRect, dstRect, null)
            }
        } catch (e: Exception) {
            android.util.Log.e("NirvanaView", "Exception drawing to canvas", e)
        } finally {
            if (canvas != null) {
                holder.unlockCanvasAndPost(canvas)
            }
        }
    }
}
