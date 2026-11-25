package com.nirvana.browser

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.SeekBar
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.nirvana.browser.data.BrowserPrefs
import com.nirvana.browser.data.BrowserStorage
import com.nirvana.browser.data.SearchEngine

/**
 * Editorial Settings Screen for Nirvana Browser with full engine, theme, zoom, privacy, and rules control.
 */
class SettingsActivity : AppCompatActivity() {

    companion object {
        const val EXTRA_CLEAR_COOKIES = "clear_cookies"
        const val EXTRA_CLEAR_STORAGE = "clear_storage"
        const val EXTRA_CLEAR_CACHE = "clear_cache"
        const val EXTRA_NEW_AD_RULE = "new_ad_rule"
    }

    private lateinit var prefs: BrowserPrefs
    private lateinit var storage: BrowserStorage

    private var clearCookiesRequested = false
    private var clearStorageRequested = false
    private var clearCacheRequested = false
    private var addedAdRule: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)

        val root = findViewById<View>(R.id.settingsRoot)
        val minTop = (34 * resources.displayMetrics.density).toInt()
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, windowInsets ->
            val insets = windowInsets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
            )
            view.setPadding(insets.left, insets.top.coerceAtLeast(minTop), insets.right, insets.bottom)
            windowInsets
        }

        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                finishWithResult()
            }
        })

        prefs = BrowserPrefs(this)
        storage = BrowserStorage(this)

        findViewById<ImageButton>(R.id.btnSettingsBack).setOnClickListener {
            finishWithResult()
        }

        bindControls()
        applySettingsTheme()
    }

    private fun bindControls() {
        val tvSearch = findViewById<TextView>(R.id.tvCurrentSearchEngine)
        tvSearch.text = prefs.searchEngine.displayName
        findViewById<View>(R.id.rowSearchEngine).setOnClickListener {
            val engines = SearchEngine.values()
            val names = engines.map { it.displayName }.toTypedArray()
            AlertDialog.Builder(this)
                .setTitle("Default Search Engine")
                .setSingleChoiceItems(names, prefs.searchEngine.ordinal) { dlg, which ->
                    prefs.searchEngine = engines[which]
                    tvSearch.text = prefs.searchEngine.displayName
                    dlg.dismiss()
                }
                .setNegativeButton("Cancel", null)
                .show()
        }

        findViewById<SwitchCompat>(R.id.switchSmartSearch).apply {
            isChecked = prefs.smartSearchFallback
            setOnCheckedChangeListener { _, checked -> prefs.smartSearchFallback = checked }
        }

        findViewById<SwitchCompat>(R.id.switchShortcuts).apply {
            isChecked = prefs.showShortcuts
            setOnCheckedChangeListener { _, checked -> prefs.showShortcuts = checked }
        }

        findViewById<SwitchCompat>(R.id.switchBottomBar).apply {
            isChecked = prefs.showBottomBar
            setOnCheckedChangeListener { _, checked -> prefs.showBottomBar = checked }
        }

        val themes = arrayOf(
            "Warm Paper" to "paper",
            "Clean Light" to "light",
            "Charcoal Dark" to "dark"
        )
        val tvTheme = findViewById<TextView>(R.id.tvCurrentTheme)
        tvTheme.text = themes.find { it.second == prefs.appTheme }?.first ?: "Warm Paper"
        findViewById<View>(R.id.rowTheme).setOnClickListener {
            val labels = themes.map { it.first }.toTypedArray()
            val cur = themes.indexOfFirst { it.second == prefs.appTheme }.coerceAtLeast(0)
            AlertDialog.Builder(this)
                .setTitle("Browser Theme")
                .setSingleChoiceItems(labels, cur) { dlg, which ->
                    prefs.appTheme = themes[which].second
                    tvTheme.text = themes[which].first
                    applySettingsTheme()
                    dlg.dismiss()
                }
                .setNegativeButton("Cancel", null)
                .show()
        }

        findViewById<SwitchCompat>(R.id.switchForceDark).apply {
            isChecked = prefs.forceDarkWeb
            setOnCheckedChangeListener { _, checked -> prefs.forceDarkWeb = checked }
        }

        val tvZoom = findViewById<TextView>(R.id.tvZoomPercent)
        val seekZoom = findViewById<SeekBar>(R.id.seekBarZoom)
        fun updateZoomUi(pct: Int) {
            val bounded = pct.coerceIn(60, 160)
            prefs.textZoomPercent = bounded
            tvZoom.text = "${bounded}%"
            seekZoom.progress = bounded - 60
        }
        tvZoom.text = "${prefs.textZoomPercent}%"
        seekZoom.progress = (prefs.textZoomPercent - 60).coerceIn(0, 100)
        seekZoom.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(sb: SeekBar?, progress: Int, fromUser: Boolean) {
                val pct = 60 + progress
                tvZoom.text = "${pct}%"
                if (fromUser) prefs.textZoomPercent = pct
            }
            override fun onStartTrackingTouch(sb: SeekBar?) {}
            override fun onStopTrackingTouch(sb: SeekBar?) {}
        })
        findViewById<View>(R.id.btnZoomMinus).setOnClickListener { updateZoomUi(prefs.textZoomPercent - 10) }
        findViewById<View>(R.id.btnZoomPlus).setOnClickListener { updateZoomUi(prefs.textZoomPercent + 10) }

        findViewById<SwitchCompat>(R.id.switchAdBlock).apply {
            isChecked = prefs.adBlockEnabled
            setOnCheckedChangeListener { _, checked -> prefs.adBlockEnabled = checked }
        }

        val tvRules = findViewById<TextView>(R.id.tvCustomRulesCount)
        tvRules.text = "${prefs.getCustomAdRules().size} custom rules configured"
        findViewById<View>(R.id.rowCustomAdRule).setOnClickListener {
            val input = EditText(this).apply {
                hint = "||ads.example.com^ or ##.ad-banner"
                setPadding(40, 28, 40, 28)
            }
            AlertDialog.Builder(this)
                .setTitle("Add Custom Filter Rule")
                .setView(input)
                .setPositiveButton("Add") { _, _ ->
                    val rule = input.text.toString().trim()
                    if (rule.isNotEmpty()) {
                        prefs.addCustomAdRule(rule)
                        addedAdRule = rule
                        tvRules.text = "${prefs.getCustomAdRules().size} custom rules configured"
                        Toast.makeText(this, "Filter rule added", Toast.LENGTH_SHORT).show()
                    }
                }
                .setNegativeButton("Cancel", null)
                .show()
        }

        findViewById<SwitchCompat>(R.id.switchHttpsOnly).apply {
            isChecked = prefs.httpsOnly
            setOnCheckedChangeListener { _, checked -> prefs.httpsOnly = checked }
        }

        findViewById<SwitchCompat>(R.id.switchDnt).apply {
            isChecked = prefs.doNotTrack
            setOnCheckedChangeListener { _, checked -> prefs.doNotTrack = checked }
        }

        findViewById<SwitchCompat>(R.id.switchJsEnabled).apply {
            isChecked = prefs.jsEnabled
            setOnCheckedChangeListener { _, checked -> prefs.jsEnabled = checked }
        }

        findViewById<SwitchCompat>(R.id.switchDesktopDefault).apply {
            isChecked = prefs.desktopModeDefault
            setOnCheckedChangeListener { _, checked -> prefs.desktopModeDefault = checked }
        }

        findViewById<View>(R.id.rowClearData).setOnClickListener {
            showClearDataDialog()
        }

        findViewById<View>(R.id.rowAbout).setOnClickListener {
            AlertDialog.Builder(this)
                .setTitle("Nirvana Browser v2.1")
                .setMessage(
                    "Sovereign Pure-Rust Web Browser Engine\n\n" +
                        "Core Crates: nirvana_html, nirvana_css, nirvana_style, nirvana_layout, nirvana_gfx, nirvana_net, nirvana_adblock\n" +
                        "Memory Safety: #![forbid(unsafe_code)] across all 12 core crates\n" +
                        "JavaScript: Boa ES2024 Sandboxed Runtime\n" +
                        "Filtering: Native Network + Cosmetic Blocker"
                )
                .setPositiveButton("OK", null)
                .show()
        }
    }

    private fun showClearDataDialog() {
        val items = arrayOf(
            "Browsing History",
            "Cookies & Site Data",
            "Web Storage (localStorage / sessionStorage)",
            "Cached Images"
        )
        val checked = booleanArrayOf(true, true, true, true)
        AlertDialog.Builder(this)
            .setTitle("Clear Browsing Data")
            .setMultiChoiceItems(items, checked) { _, which, isChecked ->
                checked[which] = isChecked
            }
            .setPositiveButton("Clear Data") { _, _ ->
                if (checked[0]) storage.clearHistory()
                if (checked[1]) clearCookiesRequested = true
                if (checked[2]) clearStorageRequested = true
                if (checked[3]) clearCacheRequested = true
                Toast.makeText(this, "Selected browsing data cleared", Toast.LENGTH_SHORT).show()
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun applySettingsTheme() {
        val isDark = prefs.appTheme == "dark"
        val bgColor = when (prefs.appTheme) {
            "dark" -> Color.parseColor("#141413")
            "light" -> Color.parseColor("#FFFFFF")
            else -> Color.parseColor("#FAF9F5")
        }
        val cardColor = when (prefs.appTheme) {
            "dark" -> Color.parseColor("#1E1D1B")
            else -> Color.WHITE
        }
        val controlFill = when (prefs.appTheme) {
            "dark" -> Color.parseColor("#272623")
            else -> Color.parseColor("#F2EFE9")
        }
        val borderColor = when (prefs.appTheme) {
            "dark" -> Color.parseColor("#353430")
            else -> Color.parseColor("#E5E0D5")
        }
        val primaryText = if (isDark) Color.parseColor("#F4F1EA") else Color.parseColor("#191918")
        val secondaryText = if (isDark) Color.parseColor("#A19E95") else Color.parseColor("#6B6860")

        WindowInsetsControllerCompat(window, window.decorView).isAppearanceLightStatusBars = !isDark
        window.statusBarColor = bgColor

        val root = findViewById<ViewGroup>(R.id.settingsRoot)
        root.setBackgroundColor(bgColor)

        val d = resources.displayMetrics.density
        fun strokeBox(fill: Int, stroke: Int, radDp: Int): GradientDrawable = GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = radDp * d
            setColor(fill)
            setStroke((1 * d).toInt().coerceAtLeast(1), stroke)
        }

        // First child of root is the header bar, second is top divider, third is ScrollView
        if (root.childCount >= 2) {
            root.getChildAt(0).setBackgroundColor(bgColor)
            root.getChildAt(1).setBackgroundColor(borderColor)
        }
        findViewById<ImageButton>(R.id.btnSettingsBack).setColorFilter(primaryText)

        fun styleTree(v: View) {
            when (v) {
                is SwitchCompat -> return
                is TextView -> {
                    val role = (v.getTag(R.id.settingsRoot) as? String) ?: run {
                        val c = v.currentTextColor
                        val r = when (c) {
                            Color.parseColor("#D97757") -> "accent"
                            Color.parseColor("#B83A3A") -> "danger"
                            Color.parseColor("#6B6860") -> "secondary"
                            else -> "primary"
                        }
                        v.setTag(R.id.settingsRoot, r)
                        r
                    }
                    when (role) {
                        "primary" -> v.setTextColor(primaryText)
                        "secondary" -> v.setTextColor(secondaryText)
                    }
                }
                is ImageView -> {
                    if (v.id != R.id.btnSettingsBack) {
                        val isShieldRow = (v.parent as? ViewGroup)?.findViewById<View>(R.id.switchAdBlock) != null
                        if (!isShieldRow) v.setColorFilter(primaryText)
                    }
                }
            }
            if (v is ViewGroup) {
                for (i in 0 until v.childCount) {
                    styleTree(v.getChildAt(i))
                }
            }
        }
        styleTree(root)

        val scroll = root.getChildAt(2) as? ScrollView
        val col = scroll?.getChildAt(0) as? ViewGroup
        if (col != null) {
            for (i in 0 until col.childCount) {
                val child = col.getChildAt(i)
                if (child is LinearLayout) {
                    child.background = strokeBox(cardColor, borderColor, 16)
                    for (j in 0 until child.childCount) {
                        val inner = child.getChildAt(j)
                        if (inner !is ViewGroup && inner.layoutParams?.height in 1..4) {
                            inner.setBackgroundColor(borderColor)
                        }
                    }
                }
            }
        }
        findViewById<View>(R.id.btnZoomMinus).background = strokeBox(controlFill, borderColor, 14)
        findViewById<View>(R.id.btnZoomPlus).background = strokeBox(controlFill, borderColor, 14)
    }

    private fun finishWithResult() {
        val data = Intent().apply {
            putExtra(EXTRA_CLEAR_COOKIES, clearCookiesRequested)
            putExtra(EXTRA_CLEAR_STORAGE, clearStorageRequested)
            putExtra(EXTRA_CLEAR_CACHE, clearCacheRequested)
            addedAdRule?.let { putExtra(EXTRA_NEW_AD_RULE, it) }
        }
        setResult(Activity.RESULT_OK, data)
        finish()
    }
}
