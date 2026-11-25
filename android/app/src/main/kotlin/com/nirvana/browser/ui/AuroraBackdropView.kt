package com.nirvana.browser.ui

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Shader
import android.util.AttributeSet
import android.view.View

/**
 * Editorial Claude-inspired architectural backdrop canvas supporting Warm Paper, Dark Charcoal,
 * and Incognito Slate palettes with restrained, minimal geometric curves.
 */
class AuroraBackdropView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    var themeMode: String = "paper"
        set(value) {
            if (field != value) {
                field = value
                invalidate()
            }
        }

    var isIncognito: Boolean = false
        set(value) {
            if (field != value) {
                field = value
                invalidate()
            }
        }

    private val bgPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val arcPaint1 = Paint(Paint.ANTI_ALIAS_FLAG)
    private val arcPaint2 = Paint(Paint.ANTI_ALIAS_FLAG)
    private val path1 = Path()
    private val path2 = Path()

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val w = width.toFloat().coerceAtLeast(1f)
        val h = height.toFloat().coerceAtLeast(1f)

        val (topColor, botColor, curve1, curve2) = when {
            isIncognito -> Palette(
                top = Color.parseColor("#16151A"),
                bot = Color.parseColor("#1C1A22"),
                c1 = Color.parseColor("#211E29"),
                c2 = Color.parseColor("#26222E")
            )
            themeMode == "dark" -> Palette(
                top = Color.parseColor("#141413"),
                bot = Color.parseColor("#191917"),
                c1 = Color.parseColor("#1E1D1B"),
                c2 = Color.parseColor("#24221F")
            )
            themeMode == "light" -> Palette(
                top = Color.parseColor("#FFFFFF"),
                bot = Color.parseColor("#FAF9F5"),
                c1 = Color.parseColor("#F5F3EC"),
                c2 = Color.parseColor("#EFECE4")
            )
            else -> Palette( // Warm Claude Ivory Paper with subtle terracotta blush
                top = Color.parseColor("#FAF9F5"),
                bot = Color.parseColor("#F5F1E9"),
                c1 = Color.parseColor("#F4ECE4"),
                c2 = Color.parseColor("#EFE9DF")
            )
        }

        bgPaint.shader = LinearGradient(0f, 0f, 0f, h, topColor, botColor, Shader.TileMode.CLAMP)
        canvas.drawRect(0f, 0f, w, h, bgPaint)

        path1.reset()
        path1.moveTo(0f, h * 0.52f)
        path1.cubicTo(w * 0.30f, h * 0.44f, w * 0.68f, h * 0.56f, w, h * 0.47f)
        path1.lineTo(w, h)
        path1.lineTo(0f, h)
        path1.close()
        arcPaint1.color = curve1
        canvas.drawPath(path1, arcPaint1)

        path2.reset()
        path2.moveTo(0f, h * 0.72f)
        path2.cubicTo(w * 0.36f, h * 0.78f, w * 0.65f, h * 0.66f, w, h * 0.74f)
        path2.lineTo(w, h)
        path2.lineTo(0f, h)
        path2.close()
        arcPaint2.color = curve2
        canvas.drawPath(path2, arcPaint2)
    }

    private data class Palette(val top: Int, val bot: Int, val c1: Int, val c2: Int)
}
