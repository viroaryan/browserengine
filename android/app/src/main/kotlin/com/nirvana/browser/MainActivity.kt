package com.nirvana.browser

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.nirvana.browser.engine.NirvanaView

/**
 * Main Browser Activity providing modern Omnibox navigation,
 * uBlock Origin shield telemetry, history management, and sovereign web rendering.
 */
class MainActivity : AppCompatActivity() {

    private lateinit var btnBack: ImageButton
    private lateinit var btnForward: ImageButton
    private lateinit var btnRefresh: ImageButton
    private lateinit var etUrl: EditText
    private lateinit var layoutShield: FrameLayout
    private lateinit var ivShield: ImageView
    private lateinit var tvBlockedBadge: TextView
    private lateinit var progressBar: ProgressBar
    private lateinit var nirvanaView: NirvanaView

    private val backStack = mutableListOf<String>()
    private val forwardStack = mutableListOf<String>()
    private var currentUrlString: String = ""

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        initViews()
        setupListeners()
        handleIncomingIntent(intent)

        // Load sovereign home page if no external intent URL
        if (currentUrlString.isEmpty()) {
            loadDefaultHome()
        }
    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        intent?.let { handleIncomingIntent(it) }
    }

    private fun initViews() {
        btnBack = findViewById(R.id.btnBack)
        btnForward = findViewById(R.id.btnForward)
        btnRefresh = findViewById(R.id.btnRefresh)
        etUrl = findViewById(R.id.etUrl)
        layoutShield = findViewById(R.id.layoutShield)
        ivShield = findViewById(R.id.ivShield)
        tvBlockedBadge = findViewById(R.id.tvBlockedBadge)
        progressBar = findViewById(R.id.progressBar)
        nirvanaView = findViewById(R.id.nirvanaView)

        updateNavButtons()
    }

    private fun setupListeners() {
        btnBack.setOnClickListener {
            if (backStack.isNotEmpty()) {
                val previous = backStack.removeAt(backStack.size - 1)
                if (currentUrlString.isNotEmpty()) {
                    forwardStack.add(currentUrlString)
                }
                loadUrlInternal(previous, isHistoryNavigation = true)
            }
        }

        btnForward.setOnClickListener {
            if (forwardStack.isNotEmpty()) {
                val next = forwardStack.removeAt(forwardStack.size - 1)
                if (currentUrlString.isNotEmpty()) {
                    backStack.add(currentUrlString)
                }
                loadUrlInternal(next, isHistoryNavigation = true)
            }
        }

        btnRefresh.setOnClickListener {
            if (currentUrlString.isNotEmpty()) {
                loadUrlInternal(currentUrlString, isHistoryNavigation = true)
            } else {
                loadDefaultHome()
            }
        }

        etUrl.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_GO || actionId == EditorInfo.IME_ACTION_DONE) {
                val query = etUrl.text.toString().trim()
                if (query.isNotEmpty()) {
                    hideKeyboard()
                    navigateTo(formatUrl(query))
                }
                true
            } else {
                false
            }
        }

        layoutShield.setOnClickListener {
            showUBlockShieldDialog()
        }

        nirvanaView.onLinkClickListener = { targetUrl ->
            runOnUiThread {
                navigateTo(targetUrl)
            }
        }
    }

    private fun navigateTo(url: String) {
        if (currentUrlString.isNotEmpty() && currentUrlString != url) {
            backStack.add(currentUrlString)
            forwardStack.clear()
        }
        loadUrlInternal(url, isHistoryNavigation = false)
    }

    private fun loadUrlInternal(url: String, isHistoryNavigation: Boolean) {
        currentUrlString = url
        etUrl.setText(url)
        updateNavButtons()
        progressBar.visibility = View.VISIBLE

        Thread {
            val success = nirvanaView.engine.navigate(url)
            runOnUiThread {
                progressBar.visibility = View.GONE
                updateShieldBadge()
                nirvanaView.requestRender()
                if (!success) {
                    Toast.makeText(this, "Failed to load page: $url", Toast.LENGTH_SHORT).show()
                }
            }
        }.start()
    }

    private fun loadDefaultHome() {
        val homeHtml = """
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body { margin: 0; padding: 24px; font-family: sans-serif; background: #ffffff; color: #202124; text-align: center; }
                    .logo { font-size: 28px; font-weight: bold; color: #1a73e8; margin-top: 36px; margin-bottom: 8px; }
                    .tagline { color: #5f6368; font-size: 14px; margin-bottom: 24px; }
                    .shield-banner { background: #e6f4ea; border: 1px solid #ceead6; border-radius: 12px; padding: 14px; margin: 16px auto; max-width: 320px; color: #137333; font-size: 13px; line-height: 1.4; }
                    .shortcuts { margin-top: 28px; }
                    .card { background: #f8f9fa; border: 1px solid #dadce0; border-radius: 8px; padding: 10px 18px; margin: 6px; display: inline-block; text-decoration: none; color: #1a73e8; font-weight: 500; font-size: 13px; }
                </style>
            </head>
            <body>
                <div class="logo">Nirvana Browser</div>
                <div class="tagline">Sovereign Pure-Rust Engine</div>
                <div class="shield-banner">🛡️ <b>uBlock Origin Active</b><br>Ads and trackers are blocked at network level.</div>
                <div class="shortcuts">
                    <a class="card" href="https://wikipedia.org">Wikipedia</a>
                    <a class="card" href="https://duckduckgo.com">DuckDuckGo</a>
                    <a class="card" href="https://news.ycombinator.com">Hacker News</a>
                </div>
            </body>
            </html>
        """.trimIndent()

        nirvanaView.loadHtml(homeHtml)
        etUrl.setText("")
        updateShieldBadge()
    }

    private fun formatUrl(input: String): String {
        return when {
            input.startsWith("http://") || input.startsWith("https://") -> input
            input.contains(".") && !input.contains(" ") -> "https://$input"
            else -> "https://duckduckgo.com/?q=" + Uri.encode(input)
        }
    }

    private fun updateShieldBadge() {
        val blockedCount = nirvanaView.engine.getBlockedRequestsCount()
        if (blockedCount > 0) {
            tvBlockedBadge.visibility = View.VISIBLE
            tvBlockedBadge.text = if (blockedCount > 99) "99+" else blockedCount.toString()
        } else {
            tvBlockedBadge.visibility = View.GONE
        }
    }

    private fun showUBlockShieldDialog() {
        val blockedCount = nirvanaView.engine.getBlockedRequestsCount()
        val isEnabled = nirvanaView.engine.isAdBlockEnabled()

        val statusText = if (isEnabled) "Active (Protecting Privacy)" else "Disabled"
        val message = "Ads & Trackers Blocked: $blockedCount\nStatus: $statusText\n\nNirvana Engine intercepts malicious ad requests before any network bytes are downloaded."

        val builder = AlertDialog.Builder(this)
            .setTitle(getString(R.string.shield_title))
            .setMessage(message)
            .setPositiveButton(if (isEnabled) "Disable AdBlock" else "Enable AdBlock") { _, _ ->
                nirvanaView.engine.setAdBlockEnabled(!isEnabled)
                Toast.makeText(
                    this,
                    if (!isEnabled) "uBlock Origin Enabled" else "uBlock Origin Disabled",
                    Toast.LENGTH_SHORT
                ).show()
                updateShieldBadge()
            }
            .setNeutralButton(getString(R.string.reset)) { _, _ ->
                nirvanaView.engine.resetBlockedCount()
                updateShieldBadge()
                Toast.makeText(this, "Counter reset", Toast.LENGTH_SHORT).show()
            }
            .setNegativeButton(getString(R.string.close), null)

        builder.show()
    }

    private fun updateNavButtons() {
        btnBack.isEnabled = backStack.isNotEmpty()
        btnBack.alpha = if (btnBack.isEnabled) 1.0f else 0.4f

        btnForward.isEnabled = forwardStack.isNotEmpty()
        btnForward.alpha = if (btnForward.isEnabled) 1.0f else 0.4f
    }

    private fun handleIncomingIntent(intent: Intent) {
        if (intent.action == Intent.ACTION_VIEW) {
            val uri = intent.data
            if (uri != null) {
                navigateTo(uri.toString())
            }
        }
    }

    private fun hideKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(etUrl.windowToken, 0)
    }

    override fun onBackPressed() {
        if (backStack.isNotEmpty()) {
            val previous = backStack.removeAt(backStack.size - 1)
            loadUrlInternal(previous, isHistoryNavigation = true)
        } else {
            super.onBackPressed()
        }
    }
}
