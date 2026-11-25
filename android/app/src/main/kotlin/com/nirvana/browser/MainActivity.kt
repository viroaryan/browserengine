package com.nirvana.browser

import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.util.TypedValue
import android.view.Gravity
import android.view.KeyEvent
import android.view.LayoutInflater
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.android.material.bottomsheet.BottomSheetDialog
import com.nirvana.browser.data.BookmarkItem
import com.nirvana.browser.data.BrowserPrefs
import com.nirvana.browser.data.BrowserStorage
import com.nirvana.browser.data.BrowserTab
import com.nirvana.browser.data.HistoryStep
import com.nirvana.browser.data.OmniboxRouter
import com.nirvana.browser.data.SearchEngine
import com.nirvana.browser.data.ShortcutItem
import com.nirvana.browser.data.SmartSearchService
import com.nirvana.browser.engine.NirvanaView
import com.nirvana.browser.ui.AuroraBackdropView
import java.net.URLEncoder
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicLong

/**
 * Editorial Chrome/Firefox/Claude-grade Android Browser UI wrapping the Sovereign Pure-Rust Nirvana Engine.
 * Supports Lock-free Async Network Prefetching, Multi-Tab Switching, Private Mode, Live Omnibox Suggestions,
 * Universal SSR Search Resolution, Bookmarks, History, Find-in-Page, Reader View, Desktop Mode, Dark Mode, & Settings.
 */
class MainActivity : AppCompatActivity() {

    private lateinit var prefs: BrowserPrefs
    private lateinit var storage: BrowserStorage

    // Top Bar & Find Bar
    private lateinit var rootLayout: LinearLayout
    private lateinit var toolbarLayout: LinearLayout
    private lateinit var omniboxContainer: LinearLayout
    private lateinit var btnOmniboxBadge: LinearLayout
    private lateinit var tvEngineBadge: TextView
    private lateinit var ivSecurityLock: ImageView
    private lateinit var etUrl: EditText
    private lateinit var btnClearUrl: ImageView
    private lateinit var layoutShield: FrameLayout
    private lateinit var ivShield: ImageView
    private lateinit var tvBlockedBadge: TextView
    private lateinit var btnTabs: FrameLayout
    private lateinit var tvTabCount: TextView
    private lateinit var btnMenu: ImageButton
    private lateinit var progressBar: ProgressBar
    private lateinit var topDivider: View

    private lateinit var findInPageBar: LinearLayout
    private lateinit var etFindQuery: EditText
    private lateinit var tvFindCount: TextView
    private lateinit var btnFindPrev: ImageButton
    private lateinit var btnFindNext: ImageButton
    private lateinit var btnFindClose: ImageButton

    // Viewports & Overlays
    private lateinit var nirvanaView: NirvanaView
    private lateinit var homeContainer: FrameLayout
    private lateinit var auroraBackdrop: AuroraBackdropView
    private lateinit var homeLayout: ScrollView
    private lateinit var btnHomePrivateToggle: LinearLayout
    private lateinit var ivPrivateToggleIcon: ImageView
    private lateinit var tvPrivateToggleLabel: TextView
    private lateinit var tvHomeBrand: TextView
    private lateinit var tvHomeSubtitle: TextView
    private lateinit var homeSearchCard: LinearLayout
    private lateinit var tvHomeSearchPlaceholder: TextView
    private lateinit var ivHomeSearchIcon: ImageView
    private lateinit var tvHomeEngineChip: TextView
    private lateinit var shortcutsSection: LinearLayout
    private lateinit var shortcutsGridContainer: LinearLayout
    private lateinit var cardShieldStatus: LinearLayout
    private lateinit var ivHomeShieldIcon: ImageView
    private lateinit var tvHomeShieldTitle: TextView
    private lateinit var tvHomeShieldSub: TextView
    private lateinit var tvHomeShieldCount: TextView
    private lateinit var suggestionsScroll: ScrollView
    private lateinit var suggestionsPanel: LinearLayout

    // Bottom Navigation Bar
    private lateinit var bottomNavBar: LinearLayout
    private lateinit var bottomDivider: View
    private lateinit var btnBack: ImageButton
    private lateinit var btnForward: ImageButton
    private lateinit var btnHome: ImageButton
    private lateinit var btnBookmark: ImageButton
    private lateinit var btnRefresh: ImageButton

    // Home Engine Pills
    private lateinit var pillGoogle: TextView
    private lateinit var pillDuckDuckGo: TextView
    private lateinit var pillBing: TextView
    private lateinit var pillBrave: TextView
    private lateinit var pillEcosia: TextView

    // Browser State
    private val tabs = mutableListOf<BrowserTab>()
    private var activeTabIndex = 0
    private var nextTabId = 1L
    private var findMatchIndex = 0
    private var lastFindTotal = 0

    private val navGeneration = AtomicLong(0L)
    @Volatile
    private var isLoading = false

    private val suggestExecutor = Executors.newSingleThreadExecutor()
    private val navExecutor = Executors.newCachedThreadPool()
    private var suggestGen = 0L

    private val settingsLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        applyPrefsToUiAndEngine()
        if (result.resultCode == Activity.RESULT_OK && result.data != null) {
            val d = result.data!!
            if (d.getBooleanExtra(SettingsActivity.EXTRA_CLEAR_COOKIES, false)) {
                nirvanaView.engine.clearCookies()
            }
            if (d.getBooleanExtra(SettingsActivity.EXTRA_CLEAR_STORAGE, false)) {
                nirvanaView.engine.clearStorage()
            }
            if (d.getBooleanExtra(SettingsActivity.EXTRA_CLEAR_CACHE, false)) {
                nirvanaView.engine.clearImageCache()
            }
            d.getStringExtra(SettingsActivity.EXTRA_NEW_AD_RULE)?.let { rule ->
                nirvanaView.engine.addAdBlockRule(rule)
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        rootLayout = findViewById(R.id.rootLayout)
        val minTop = (34 * resources.displayMetrics.density).toInt()
        ViewCompat.setOnApplyWindowInsetsListener(rootLayout) { view, windowInsets ->
            val insets = windowInsets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
            )
            view.setPadding(insets.left, insets.top.coerceAtLeast(minTop), insets.right, insets.bottom)
            windowInsets
        }

        prefs = BrowserPrefs(this)
        storage = BrowserStorage(this)

        initViews()
        setupListeners()
        setupBackDispatcher()
        initFirstTab()
        applyPrefsToUiAndEngine()
        handleIncomingIntent(intent)
    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        setIntent(intent)
        intent?.let { handleIncomingIntent(it) }
    }

    override fun onDestroy() {
        navGeneration.incrementAndGet()
        suggestExecutor.shutdownNow()
        navExecutor.shutdownNow()
        if (::nirvanaView.isInitialized) {
            nirvanaView.destroyEngine()
        }
        super.onDestroy()
    }

    private fun setupBackDispatcher() {
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (findInPageBar.visibility == View.VISIBLE) {
                    findInPageBar.visibility = View.GONE
                    etFindQuery.setText("")
                    return
                }
                if (suggestionsScroll.visibility == View.VISIBLE) {
                    suggestionsScroll.visibility = View.GONE
                    etUrl.clearFocus()
                    return
                }
                val tab = currentTab()
                if (tab.backStack.isNotEmpty() || !tab.isOnHome) {
                    navigateBack()
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                    isEnabled = true
                }
            }
        })
    }

    private fun initViews() {
        toolbarLayout = findViewById(R.id.toolbarLayout)
        omniboxContainer = findViewById(R.id.omniboxContainer)
        btnOmniboxBadge = findViewById(R.id.btnOmniboxBadge)
        tvEngineBadge = findViewById(R.id.tvEngineBadge)
        ivSecurityLock = findViewById(R.id.ivSecurityLock)
        etUrl = findViewById(R.id.etUrl)
        btnClearUrl = findViewById(R.id.btnClearUrl)
        layoutShield = findViewById(R.id.layoutShield)
        ivShield = findViewById(R.id.ivShield)
        tvBlockedBadge = findViewById(R.id.tvBlockedBadge)
        btnTabs = findViewById(R.id.btnTabs)
        tvTabCount = findViewById(R.id.tvTabCount)
        btnMenu = findViewById(R.id.btnMenu)
        progressBar = findViewById(R.id.progressBar)
        topDivider = findViewById(R.id.topDivider)

        findInPageBar = findViewById(R.id.findInPageBar)
        etFindQuery = findViewById(R.id.etFindQuery)
        tvFindCount = findViewById(R.id.tvFindCount)
        btnFindPrev = findViewById(R.id.btnFindPrev)
        btnFindNext = findViewById(R.id.btnFindNext)
        btnFindClose = findViewById(R.id.btnFindClose)

        nirvanaView = findViewById(R.id.nirvanaView)
        homeContainer = findViewById(R.id.homeContainer)
        auroraBackdrop = findViewById(R.id.auroraBackdrop)
        homeLayout = findViewById(R.id.homeLayout)
        btnHomePrivateToggle = findViewById(R.id.btnHomePrivateToggle)
        ivPrivateToggleIcon = findViewById(R.id.ivPrivateToggleIcon)
        tvPrivateToggleLabel = findViewById(R.id.tvPrivateToggleLabel)
        tvHomeBrand = findViewById(R.id.tvHomeBrand)
        tvHomeSubtitle = findViewById(R.id.tvHomeSubtitle)
        homeSearchCard = findViewById(R.id.homeSearchCard)
        ivHomeSearchIcon = findViewById(R.id.ivHomeSearchIcon)
        tvHomeSearchPlaceholder = findViewById(R.id.tvHomeSearchPlaceholder)
        tvHomeEngineChip = findViewById(R.id.tvHomeEngineChip)
        shortcutsSection = findViewById(R.id.shortcutsSection)
        shortcutsGridContainer = findViewById(R.id.shortcutsGridContainer)
        cardShieldStatus = findViewById(R.id.cardShieldStatus)
        ivHomeShieldIcon = findViewById(R.id.ivHomeShieldIcon)
        tvHomeShieldTitle = findViewById(R.id.tvHomeShieldTitle)
        tvHomeShieldSub = findViewById(R.id.tvHomeShieldSub)
        tvHomeShieldCount = findViewById(R.id.tvHomeShieldCount)
        suggestionsScroll = findViewById(R.id.suggestionsScroll)
        suggestionsPanel = findViewById(R.id.suggestionsPanel)

        bottomNavBar = findViewById(R.id.bottomNavBar)
        bottomDivider = findViewById(R.id.bottomDivider)
        btnBack = findViewById(R.id.btnBack)
        btnForward = findViewById(R.id.btnForward)
        btnHome = findViewById(R.id.btnHome)
        btnBookmark = findViewById(R.id.btnBookmark)
        btnRefresh = findViewById(R.id.btnRefresh)

        pillGoogle = findViewById(R.id.pillGoogle)
        pillDuckDuckGo = findViewById(R.id.pillDuckDuckGo)
        pillBing = findViewById(R.id.pillBing)
        pillBrave = findViewById(R.id.pillBrave)
        pillEcosia = findViewById(R.id.pillEcosia)
    }

    private fun initFirstTab() {
        tabs.clear()
        tabs.add(
            BrowserTab(
                id = nextTabId++,
                title = "Home",
                isOnHome = true,
                desktopMode = prefs.desktopModeDefault
            )
        )
        activeTabIndex = 0
        updateTabUiState()
    }

    private fun currentTab(): BrowserTab = tabs[activeTabIndex.coerceIn(0, tabs.lastIndex)]

    private fun applyPrefsToUiAndEngine() {
        nirvanaView.userZoomFactor = prefs.textZoomPercent / 100f
        nirvanaView.engine.setAdBlockEnabled(prefs.adBlockEnabled)
        for (rule in prefs.getCustomAdRules()) {
            nirvanaView.engine.addAdBlockRule(rule)
        }
        val tab = currentTab()
        nirvanaView.engine.configure(
            desktopMode = tab.desktopMode,
            httpsOnly = prefs.httpsOnly,
            jsEnabled = prefs.jsEnabled,
            forceDark = prefs.forceDarkWeb,
            doNotTrack = prefs.doNotTrack
        )
        shortcutsSection.visibility = if (prefs.showShortcuts) View.VISIBLE else View.GONE
        bottomNavBar.visibility = if (prefs.showBottomBar) View.VISIBLE else View.GONE

        applyThemeStyling()
        updateEnginePillsUi()
        updateSearchHints()
        rebuildHomeShortcutsGrid()
        updateShieldBadge()
        if (!tab.isOnHome) {
            nirvanaView.requestRender()
        }
    }

    private fun applyThemeStyling() {
        val tab = currentTab()
        auroraBackdrop.themeMode = prefs.appTheme
        auroraBackdrop.isIncognito = tab.isIncognito

        val darkChrome = tab.isIncognito || prefs.appTheme == "dark"
        val barColor = when {
            tab.isIncognito -> Color.parseColor("#16151A")
            prefs.appTheme == "dark" -> Color.parseColor("#141413")
            prefs.appTheme == "light" -> Color.parseColor("#FFFFFF")
            else -> Color.parseColor("#FAF9F5")
        }
        val surfaceColor = when {
            tab.isIncognito -> Color.parseColor("#22202A")
            prefs.appTheme == "dark" -> Color.parseColor("#1E1D1B")
            else -> Color.parseColor("#FFFFFF")
        }
        val controlFill = when {
            tab.isIncognito -> Color.parseColor("#2A2835")
            prefs.appTheme == "dark" -> Color.parseColor("#272623")
            else -> Color.parseColor("#F2EFE9")
        }
        val borderColor = when {
            tab.isIncognito -> Color.parseColor("#353242")
            prefs.appTheme == "dark" -> Color.parseColor("#353430")
            else -> Color.parseColor("#E5E0D5")
        }

        val primaryText = if (darkChrome) Color.parseColor("#F4F1EA") else Color.parseColor("#191918")
        val secondaryText = if (darkChrome) Color.parseColor("#A19E95") else Color.parseColor("#6B6860")

        WindowInsetsControllerCompat(window, window.decorView).isAppearanceLightStatusBars = !darkChrome
        window.statusBarColor = barColor

        rootLayout.setBackgroundColor(barColor)
        toolbarLayout.setBackgroundColor(barColor)
        bottomNavBar.setBackgroundColor(barColor)
        topDivider.setBackgroundColor(borderColor)
        bottomDivider.setBackgroundColor(borderColor)

        omniboxContainer.background = createStrokeBox(controlFill, borderColor, 22)
        homeSearchCard.background = createStrokeBox(surfaceColor, borderColor, 24)
        suggestionsScroll.background = createStrokeBox(surfaceColor, borderColor, 16)
        cardShieldStatus.background = createStrokeBox(surfaceColor, borderColor, 16)
        tvHomeShieldCount.background = createStrokeBox(controlFill, borderColor, 12)
        btnHomePrivateToggle.background = createStrokeBox(
            if (tab.isIncognito) Color.parseColor("#D97757") else surfaceColor,
            if (tab.isIncognito) Color.parseColor("#D97757") else borderColor,
            18
        )

        findInPageBar.setBackgroundColor(controlFill)
        etFindQuery.setTextColor(primaryText)
        etFindQuery.setHintTextColor(secondaryText)
        tvFindCount.setTextColor(secondaryText)
        btnFindPrev.setColorFilter(primaryText)
        btnFindNext.setColorFilter(primaryText)
        btnFindClose.setColorFilter(primaryText)

        etUrl.setTextColor(primaryText)
        etUrl.setHintTextColor(secondaryText)
        tvTabCount.setTextColor(primaryText)
        tvTabCount.background = createOutlineStrokeBox(primaryText, 6)

        tvHomeBrand.setTextColor(primaryText)
        tvHomeSubtitle.setTextColor(if (tab.isIncognito) Color.parseColor("#D97757") else secondaryText)
        tvHomeSearchPlaceholder.setTextColor(secondaryText)
        ivHomeSearchIcon.setColorFilter(secondaryText)

        tvPrivateToggleLabel.text = if (tab.isIncognito) "Exit Private" else "Private"
        val toggleInk = if (tab.isIncognito) Color.WHITE else primaryText
        tvPrivateToggleLabel.setTextColor(toggleInk)
        ivPrivateToggleIcon.setColorFilter(toggleInk)

        tvHomeShieldTitle.setTextColor(primaryText)
        tvHomeShieldSub.setTextColor(secondaryText)

        btnBack.setColorFilter(primaryText)
        btnForward.setColorFilter(primaryText)
        btnHome.setColorFilter(primaryText)
        btnRefresh.setColorFilter(primaryText)
        btnMenu.setColorFilter(primaryText)
        btnClearUrl.setColorFilter(secondaryText)
        btnBookmark.clearColorFilter()
        btnBookmark.setColorFilter(primaryText)
        ivHomeShieldIcon.setColorFilter(
            if (nirvanaView.engine.isAdBlockEnabled()) Color.parseColor("#2F6F4E") else secondaryText
        )
        ivShield.setColorFilter(
            if (nirvanaView.engine.isAdBlockEnabled()) Color.parseColor("#2F6F4E") else secondaryText
        )
    }

    private fun setupListeners() {
        btnOmniboxBadge.setOnClickListener {
            if (currentTab().isOnHome) {
                showSearchEngineSheet()
            } else {
                showSiteSecurityDialog()
            }
        }

        homeSearchCard.setOnClickListener {
            etUrl.requestFocus()
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            imm?.showSoftInput(etUrl, InputMethodManager.SHOW_IMPLICIT)
        }

        tvHomeEngineChip.setOnClickListener {
            showSearchEngineSheet()
        }

        btnHomePrivateToggle.setOnClickListener {
            if (currentTab().isIncognito) {
                val normalIdx = tabs.indexOfFirst { !it.isIncognito }
                if (normalIdx >= 0) {
                    closeActiveTab()
                } else {
                    createNewTab(isIncognito = false)
                }
            } else {
                createNewTab(isIncognito = true)
            }
        }

        btnTabs.setOnClickListener { showTabsSheet() }
        btnMenu.setOnClickListener { showOverflowSheet() }
        layoutShield.setOnClickListener { showShieldDialog() }
        cardShieldStatus.setOnClickListener { showShieldDialog() }

        btnBack.setOnClickListener { navigateBack() }
        btnForward.setOnClickListener { navigateForward() }
        btnHome.setOnClickListener {
            stopLoading()
            showHomeScreen()
        }
        btnRefresh.setOnClickListener {
            if (isLoading) {
                stopLoading()
                Toast.makeText(this, "Stopped loading", Toast.LENGTH_SHORT).show()
            } else {
                reloadCurrentTab()
            }
        }
        btnBookmark.setOnClickListener {
            if (currentTab().isOnHome) {
                showBookmarksSheet()
            } else {
                toggleCurrentBookmark()
            }
        }

        btnClearUrl.setOnClickListener {
            etUrl.setText("")
            btnClearUrl.visibility = View.GONE
            suggestionsScroll.visibility = View.GONE
        }

        etUrl.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                btnClearUrl.visibility = if (s.isNullOrEmpty() || !etUrl.hasFocus()) View.GONE else View.VISIBLE
                if (etUrl.hasFocus()) {
                    scheduleOmniboxSuggestions(s?.toString() ?: "")
                }
            }
            override fun afterTextChanged(s: Editable?) {}
        })

        etUrl.setOnFocusChangeListener { _, hasFocus ->
            if (hasFocus) {
                if (etUrl.text.isNotEmpty()) etUrl.selectAll()
                btnClearUrl.visibility = if (etUrl.text.isNotEmpty()) View.VISIBLE else View.GONE
                scheduleOmniboxSuggestions(etUrl.text.toString())
            } else {
                btnClearUrl.visibility = View.GONE
                suggestionsScroll.visibility = View.GONE
            }
        }

        etUrl.setOnEditorActionListener { _, actionId, event ->
            val isEnter = event != null && event.keyCode == KeyEvent.KEYCODE_ENTER &&
                (event.action == KeyEvent.ACTION_DOWN || event.action == KeyEvent.ACTION_UP)
            val isSearchAction = actionId == EditorInfo.IME_ACTION_SEARCH ||
                actionId == EditorInfo.IME_ACTION_GO ||
                actionId == EditorInfo.IME_ACTION_DONE ||
                actionId == EditorInfo.IME_ACTION_SEND ||
                actionId == EditorInfo.IME_ACTION_UNSPECIFIED

            if (isSearchAction || isEnter) {
                submitCurrentOmnibox()
                true
            } else {
                false
            }
        }

        etUrl.setOnKeyListener { _, keyCode, event ->
            if ((keyCode == KeyEvent.KEYCODE_ENTER || keyCode == KeyEvent.KEYCODE_NUMPAD_ENTER) &&
                event.action == KeyEvent.ACTION_UP
            ) {
                submitCurrentOmnibox()
                true
            } else {
                false
            }
        }

        pillGoogle.setOnClickListener { selectSearchEngine(SearchEngine.GOOGLE) }
        pillDuckDuckGo.setOnClickListener { selectSearchEngine(SearchEngine.DUCKDUCKGO) }
        pillBing.setOnClickListener { selectSearchEngine(SearchEngine.BING) }
        pillBrave.setOnClickListener { selectSearchEngine(SearchEngine.BRAVE) }
        pillEcosia.setOnClickListener { selectSearchEngine(SearchEngine.ECOSIA) }

        etFindQuery.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                findMatchIndex = 0
                runFindInPage()
            }
            override fun afterTextChanged(s: Editable?) {}
        })
        btnFindNext.setOnClickListener {
            if (lastFindTotal > 0) {
                findMatchIndex = (findMatchIndex + 1) % lastFindTotal
                runFindInPage()
            }
        }
        btnFindPrev.setOnClickListener {
            if (lastFindTotal > 0) {
                findMatchIndex = if (findMatchIndex == 0) lastFindTotal - 1 else findMatchIndex - 1
                runFindInPage()
            }
        }
        btnFindClose.setOnClickListener {
            findInPageBar.visibility = View.GONE
            etFindQuery.setText("")
            hideKeyboard()
        }

        nirvanaView.onLinkClickListener = { rawUrl ->
            val clean = SmartSearchService.unwrapSearchUrl(rawUrl)
            runOnUiThread {
                navigateTo(clean)
            }
        }
    }

    private fun submitCurrentOmnibox() {
        val query = etUrl.text.toString().trim()
        if (query.isEmpty()) return
        hideKeyboard()
        suggestionsScroll.visibility = View.GONE
        etUrl.clearFocus()
        submitOmniboxInput(query)
    }

    private fun submitOmniboxInput(input: String) {
        val trimmed = input.trim()
        if (OmniboxRouter.looksLikeUrl(trimmed)) {
            navigateTo(OmniboxRouter.normalizeUrl(trimmed))
        } else {
            performSearch(trimmed)
        }
    }

    private fun looksLikeUrl(input: String): Boolean = OmniboxRouter.looksLikeUrl(input)

    private fun stopLoading() {
        navGeneration.incrementAndGet()
        isLoading = false
        progressBar.visibility = View.GONE
        btnRefresh.setImageResource(R.drawable.ic_refresh)
    }

    private fun setLoadingState(loading: Boolean) {
        isLoading = loading
        progressBar.visibility = if (loading) View.VISIBLE else View.GONE
        btnRefresh.setImageResource(if (loading) R.drawable.ic_stop else R.drawable.ic_refresh)
    }

    private fun pushCurrentToBackStack(tab: BrowserTab) {
        if (!tab.isOnHome && tab.url.isNotEmpty()) {
            tab.backStack.add(
                HistoryStep(
                    url = tab.url,
                    isSearchSerp = tab.isSearchSerp,
                    searchQuery = tab.searchQuery,
                    cachedHtml = tab.cachedHtml
                )
            )
            tab.forwardStack.clear()
        }
    }

    private fun performSearch(query: String, recordHistoryStep: Boolean = true) {
        val tab = currentTab()
        val engine = prefs.searchEngine
        val targetUrl = engine.searchUrl + URLEncoder.encode(query, "UTF-8")

        if (recordHistoryStep && !(tab.isSearchSerp && tab.searchQuery == query)) {
            pushCurrentToBackStack(tab)
        }

        tab.isOnHome = false
        tab.isSearchSerp = true
        tab.searchQuery = query
        tab.url = targetUrl
        tab.title = "$query - ${engine.displayName}"
        tab.readerModeActive = false
        tab.cachedHtml = null

        val myGen = navGeneration.incrementAndGet()
        val targetTabId = tab.id

        etUrl.setText(query)
        etUrl.clearFocus()
        suggestionsScroll.visibility = View.GONE
        updateTabUiState()
        setLoadingState(true)

        navExecutor.submit {
            nirvanaView.engine.configure(
                desktopMode = tab.desktopMode,
                httpsOnly = prefs.httpsOnly,
                jsEnabled = prefs.jsEnabled,
                forceDark = prefs.forceDarkWeb,
                doNotTrack = prefs.doNotTrack
            )
            val snapPtr = nirvanaView.engine.beginNavPrefetch(targetUrl)
            val resPtr = nirvanaView.engine.executeNavPrefetch(snapPtr, targetUrl)
            val stillCurrent = (navGeneration.get() == myGen && currentTab().id == targetTabId)
            val navOk = nirvanaView.engine.commitNavResult(resPtr, stillCurrent)
            if (!stillCurrent) return@submit

            val snippet = if (navOk) nirvanaView.engine.getPageTextSnippet(512) else ""
            val resolvedUrl = nirvanaView.engine.currentUrl ?: targetUrl
            val useFallback = prefs.smartSearchFallback &&
                (!navOk || SmartSearchService.isSearchWallOrEmpty(engine, resolvedUrl, snippet))

            if (useFallback) {
                val serpHtml = SmartSearchService.fetchFormattedChromeSerpHtml(query, engine)
                if (navGeneration.get() != myGen || currentTab().id != targetTabId) return@submit
                tab.cachedHtml = serpHtml
                runOnUiThread {
                    if (navGeneration.get() != myGen || currentTab().id != targetTabId) return@runOnUiThread
                    setLoadingState(false)
                    nirvanaView.loadHtml(serpHtml, "")
                    tab.url = targetUrl
                    etUrl.setText(query)
                    if (!tab.isIncognito) {
                        storage.recordHistory(tab.title, targetUrl)
                    }
                    updateShieldBadge()
                    updateTabUiState()
                }
            } else {
                val err = if (!navOk) nirvanaView.engine.lastError else null
                val title = nirvanaView.engine.pageTitle ?: "$query - ${engine.displayName}"
                runOnUiThread {
                    if (navGeneration.get() != myGen || currentTab().id != targetTabId) return@runOnUiThread
                    setLoadingState(false)
                    updateShieldBadge()
                    if (navOk) {
                        nirvanaView.markContentLoaded()
                        tab.url = resolvedUrl
                        tab.title = title
                        etUrl.setText(resolvedUrl)
                        nirvanaView.requestRender()
                        if (!tab.isIncognito) {
                            storage.recordHistory(title, resolvedUrl)
                        }
                    } else {
                        val errorHtml = buildErrorPageHtml(targetUrl, err ?: "Network error")
                        nirvanaView.loadHtml(errorHtml, "")
                        Toast.makeText(this, "Search failed: ${err ?: "Network error"}", Toast.LENGTH_LONG).show()
                    }
                    updateTabUiState()
                }
            }
        }
    }

    private fun navigateTo(url: String) {
        val tab = currentTab()
        if (!tab.isOnHome && tab.url != url) {
            pushCurrentToBackStack(tab)
        }
        loadUrlInternal(url)
    }

    private fun loadUrlInternal(url: String) {
        val tab = currentTab()
        tab.isOnHome = false
        tab.isSearchSerp = false
        tab.searchQuery = ""
        tab.cachedHtml = null
        tab.readerModeActive = false
        tab.url = url

        val myGen = navGeneration.incrementAndGet()
        val targetTabId = tab.id

        etUrl.setText(url)
        etUrl.clearFocus()
        suggestionsScroll.visibility = View.GONE
        updateTabUiState()
        setLoadingState(true)

        navExecutor.submit {
            nirvanaView.engine.configure(
                desktopMode = tab.desktopMode,
                httpsOnly = prefs.httpsOnly,
                jsEnabled = prefs.jsEnabled,
                forceDark = prefs.forceDarkWeb,
                doNotTrack = prefs.doNotTrack
            )
            val snapPtr = nirvanaView.engine.beginNavPrefetch(url)
            val resPtr = nirvanaView.engine.executeNavPrefetch(snapPtr, url)
            val stillCurrent = (navGeneration.get() == myGen && currentTab().id == targetTabId)
            val success = nirvanaView.engine.commitNavResult(resPtr, stillCurrent)
            if (!stillCurrent) return@submit

            val navError = if (!success) nirvanaView.engine.lastError else null
            val actualUrl = nirvanaView.engine.currentUrl ?: url
            val docTitle = nirvanaView.engine.pageTitle ?: (Uri.parse(actualUrl).host ?: actualUrl)

            runOnUiThread {
                if (navGeneration.get() != myGen || currentTab().id != targetTabId) return@runOnUiThread
                setLoadingState(false)
                updateShieldBadge()
                if (success) {
                    nirvanaView.markContentLoaded()
                    tab.url = actualUrl
                    tab.title = docTitle
                    etUrl.setText(actualUrl)
                    nirvanaView.requestRender()
                    if (!tab.isIncognito) {
                        storage.recordHistory(docTitle, actualUrl)
                    }
                } else {
                    val errorHtml = buildErrorPageHtml(url, navError ?: "Connection failed")
                    nirvanaView.loadHtml(errorHtml, "")
                    Toast.makeText(this, "Unable to load: ${navError ?: "Network error"}", Toast.LENGTH_SHORT).show()
                }
                updateTabUiState()
            }
        }
    }

    private fun restoreHistoryStep(step: HistoryStep) {
        val tab = currentTab()
        if (step.cachedHtml != null) {
            tab.isOnHome = false
            tab.isSearchSerp = step.isSearchSerp
            tab.searchQuery = step.searchQuery
            tab.url = step.url
            tab.cachedHtml = step.cachedHtml
            tab.readerModeActive = false
            etUrl.setText(if (step.isSearchSerp) step.searchQuery else step.url)
            updateTabUiState()
            nirvanaView.loadHtml(step.cachedHtml, "")
        } else if (step.isSearchSerp && step.searchQuery.isNotEmpty()) {
            performSearch(step.searchQuery, recordHistoryStep = false)
        } else {
            loadUrlInternal(step.url)
        }
    }

    private fun reloadCurrentTab() {
        val tab = currentTab()
        if (tab.isOnHome) {
            showHomeScreen()
        } else if (tab.isSearchSerp && tab.searchQuery.isNotEmpty()) {
            performSearch(tab.searchQuery, recordHistoryStep = false)
        } else if (tab.url.isNotEmpty()) {
            loadUrlInternal(tab.url)
        }
    }

    private fun navigateBack() {
        stopLoading()
        val tab = currentTab()
        if (tab.readerModeActive) {
            tab.readerModeActive = false
            reloadCurrentTab()
            return
        }
        if (tab.backStack.isNotEmpty()) {
            val prev = tab.backStack.removeAt(tab.backStack.size - 1)
            if (!tab.isOnHome && tab.url.isNotEmpty()) {
                tab.forwardStack.add(
                    HistoryStep(tab.url, tab.isSearchSerp, tab.searchQuery, tab.cachedHtml)
                )
            }
            restoreHistoryStep(prev)
        } else if (!tab.isOnHome) {
            if (tab.url.isNotEmpty()) {
                tab.forwardStack.add(
                    HistoryStep(tab.url, tab.isSearchSerp, tab.searchQuery, tab.cachedHtml)
                )
            }
            showHomeScreen()
        }
    }

    private fun navigateForward() {
        stopLoading()
        val tab = currentTab()
        if (tab.forwardStack.isNotEmpty()) {
            val next = tab.forwardStack.removeAt(tab.forwardStack.size - 1)
            if (!tab.isOnHome && tab.url.isNotEmpty()) {
                tab.backStack.add(
                    HistoryStep(tab.url, tab.isSearchSerp, tab.searchQuery, tab.cachedHtml)
                )
            }
            restoreHistoryStep(next)
        }
    }

    fun showHomeScreen() {
        val tab = currentTab()
        tab.isOnHome = true
        tab.isSearchSerp = false
        tab.url = ""
        tab.title = if (tab.isIncognito) "Private Tab" else "New Tab"
        tab.readerModeActive = false
        etUrl.setText("")
        etUrl.clearFocus()
        suggestionsScroll.visibility = View.GONE
        findInPageBar.visibility = View.GONE
        updateTabUiState()
        updateShieldBadge()
    }

    private fun updateTabUiState() {
        val tab = currentTab()
        if (tab.isOnHome) {
            homeContainer.visibility = View.VISIBLE
            nirvanaView.visibility = View.GONE
            tvEngineBadge.visibility = View.VISIBLE
            ivSecurityLock.visibility = View.GONE
            tvEngineBadge.text = prefs.searchEngine.shortCode
            tvEngineBadge.background = createCircleBadgeDrawable(Color.parseColor("#D97757"))
        } else {
            homeContainer.visibility = View.GONE
            nirvanaView.visibility = View.VISIBLE
            tvEngineBadge.visibility = View.GONE
            ivSecurityLock.visibility = View.VISIBLE
            val isHttps = tab.url.startsWith("https://")
            ivSecurityLock.setImageResource(if (isHttps) R.drawable.ic_lock else R.drawable.ic_lock_open)
            ivSecurityLock.setColorFilter(
                if (isHttps) Color.parseColor("#2F6F4E") else Color.parseColor("#B83A3A")
            )
        }

        tvTabCount.text = tabs.size.toString()
        val canGoBack = tab.backStack.isNotEmpty() || !tab.isOnHome
        btnBack.isEnabled = canGoBack
        btnBack.alpha = if (canGoBack) 1.0f else 0.32f

        btnForward.isEnabled = tab.forwardStack.isNotEmpty()
        btnForward.alpha = if (btnForward.isEnabled) 1.0f else 0.32f

        val isBookmarked = !tab.isOnHome && storage.isBookmarked(tab.url)
        btnBookmark.setImageResource(if (isBookmarked) R.drawable.ic_star_filled else R.drawable.ic_star_outline)
        btnBookmark.alpha = 1.0f

        applyThemeStyling()
        if (isBookmarked) {
            btnBookmark.setColorFilter(Color.parseColor("#D97757"))
        }
    }

    private fun selectSearchEngine(engine: SearchEngine) {
        prefs.searchEngine = engine
        updateEnginePillsUi()
        updateSearchHints()
        updateTabUiState()
        Toast.makeText(this, "Search engine set to ${engine.displayName}", Toast.LENGTH_SHORT).show()
    }

    private fun updateEnginePillsUi() {
        val active = prefs.searchEngine
        val darkChrome = currentTab().isIncognito || prefs.appTheme == "dark"
        val pills = listOf(
            pillGoogle to SearchEngine.GOOGLE,
            pillDuckDuckGo to SearchEngine.DUCKDUCKGO,
            pillBing to SearchEngine.BING,
            pillBrave to SearchEngine.BRAVE,
            pillEcosia to SearchEngine.ECOSIA
        )
        for ((tv, eng) in pills) {
            val isSel = eng == active
            if (isSel) {
                tv.background = createStrokeBox(Color.parseColor("#D97757"), Color.parseColor("#D97757"), 18)
                tv.setTextColor(Color.WHITE)
                tv.typeface = Typeface.DEFAULT_BOLD
            } else {
                tv.background = createStrokeBox(
                    if (darkChrome) Color.parseColor("#1E1D1B") else Color.WHITE,
                    if (darkChrome) Color.parseColor("#353430") else Color.parseColor("#E5E0D5"),
                    18
                )
                tv.setTextColor(if (darkChrome) Color.parseColor("#F4F1EA") else Color.parseColor("#191918"))
                tv.typeface = Typeface.DEFAULT
            }
        }
    }

    private fun updateSearchHints() {
        val name = prefs.searchEngine.displayName
        etUrl.hint = if (currentTab().isIncognito) "Private search ($name) or URL" else "Search with $name or enter URL"
        tvHomeSearchPlaceholder.text = "Search with $name or enter URL"
        tvHomeEngineChip.text = name
        tvHomeEngineChip.background = createStrokeBox(Color.parseColor("#D97757"), Color.parseColor("#D97757"), 14)
    }

    private fun updateShieldBadge() {
        val blockedCount = nirvanaView.engine.getBlockedRequestsCount()
        if (blockedCount > 0) {
            tvBlockedBadge.visibility = View.VISIBLE
            tvBlockedBadge.text = if (blockedCount > 99) "99+" else blockedCount.toString()
        } else {
            tvBlockedBadge.visibility = View.GONE
        }
        val active = nirvanaView.engine.isAdBlockEnabled()
        tvHomeShieldTitle.text = if (active) "Tracker Protection Active" else "Tracker Protection Paused"
        tvHomeShieldCount.text = "$blockedCount blocked"
    }

    private fun shortcutDrawableRes(key: String): Int = when (key) {
        "google" -> R.drawable.ic_site_google
        "wiki" -> R.drawable.ic_site_wiki
        "hn" -> R.drawable.ic_site_hn
        "github" -> R.drawable.ic_site_github
        "youtube" -> R.drawable.ic_site_youtube
        "reddit" -> R.drawable.ic_site_reddit
        "ddg" -> R.drawable.ic_site_ddg
        "claude" -> R.drawable.ic_site_claude
        else -> R.drawable.ic_globe
    }

    private fun rebuildHomeShortcutsGrid() {
        shortcutsGridContainer.removeAllViews()
        val items = storage.getShortcuts()
        val displayItems = items.take(7).toMutableList()
        val darkChrome = currentTab().isIncognito || prefs.appTheme == "dark"

        val tileFill = if (darkChrome) Color.parseColor("#1E1D1B") else Color.WHITE
        val tileBorder = if (darkChrome) Color.parseColor("#353430") else Color.parseColor("#E5E0D5")
        val labelColor = if (darkChrome) Color.parseColor("#E6E2D8") else Color.parseColor("#191918")

        val rows = (displayItems.size + 1 + 3) / 4
        var idx = 0
        for (r in 0 until rows) {
            val rowLayout = LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = if (r > 0) dp(12) else 0 }
            }
            for (c in 0 until 4) {
                val cell = if (idx < displayItems.size) {
                    buildShortcutCell(displayItems[idx], tileFill, tileBorder, labelColor, darkChrome)
                } else if (idx == displayItems.size) {
                    buildAddShortcutCell(tileFill, tileBorder, labelColor, darkChrome)
                } else {
                    LinearLayout(this).apply {
                        layoutParams = LinearLayout.LayoutParams(0, 0, 1f)
                    }
                }
                rowLayout.addView(cell)
                idx++
            }
            shortcutsGridContainer.addView(rowLayout)
        }
    }

    private fun buildShortcutCell(
        item: ShortcutItem,
        tileFill: Int,
        tileBorder: Int,
        labelColor: Int,
        darkChrome: Boolean
    ): View {
        val wrap = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            setPadding(dp(4), dp(4), dp(4), dp(4))
            setOnClickListener { navigateTo(item.url) }
            setOnLongClickListener {
                AlertDialog.Builder(this@MainActivity)
                    .setTitle(item.title)
                    .setMessage(item.url)
                    .setPositiveButton("Open") { _, _ -> navigateTo(item.url) }
                    .setNegativeButton("Remove") { _, _ ->
                        val updated = storage.getShortcuts().filterNot { it.url == item.url }
                        storage.saveShortcuts(updated)
                        rebuildHomeShortcutsGrid()
                    }
                    .setNeutralButton("Cancel", null)
                    .show()
                true
            }
        }

        val iconBox = ImageView(this).apply {
            val size = dp(52)
            layoutParams = LinearLayout.LayoutParams(size, size)
            background = createStrokeBox(tileFill, tileBorder, 16)
            setPadding(dp(14), dp(14), dp(14), dp(14))
            val resId = shortcutDrawableRes(item.iconKey)
            setImageResource(resId)
            if (darkChrome && (item.iconKey == "google" || item.iconKey == "wiki" || item.iconKey == "github" || item.iconKey == "globe")) {
                setColorFilter(Color.parseColor("#F4F1EA"))
            }
        }

        val label = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(6) }
            text = item.title
            maxLines = 1
            textSize = 11.5f
            setTextColor(labelColor)
        }

        wrap.addView(iconBox)
        wrap.addView(label)
        return wrap
    }

    private fun buildAddShortcutCell(
        tileFill: Int,
        tileBorder: Int,
        labelColor: Int,
        darkChrome: Boolean
    ): View {
        val wrap = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            setPadding(dp(4), dp(4), dp(4), dp(4))
            setOnClickListener { showAddShortcutDialog() }
        }
        val iconBox = ImageView(this).apply {
            val size = dp(52)
            layoutParams = LinearLayout.LayoutParams(size, size)
            background = createStrokeBox(tileFill, tileBorder, 16)
            setPadding(dp(15), dp(15), dp(15), dp(15))
            setImageResource(R.drawable.ic_add)
            if (darkChrome) setColorFilter(Color.parseColor("#A19E95"))
        }
        val label = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(6) }
            text = "Add site"
            textSize = 11.5f
            setTextColor(labelColor)
        }
        wrap.addView(iconBox)
        wrap.addView(label)
        return wrap
    }

    private fun showAddShortcutDialog(prefillTitle: String = "", prefillUrl: String = "") {
        val container = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(14), dp(20), dp(6))
        }
        val etName = EditText(this).apply {
            hint = "Title (e.g. Claude)"
            setText(prefillTitle)
        }
        val etLink = EditText(this).apply {
            hint = "URL (e.g. https://claude.ai)"
            setText(prefillUrl)
        }
        container.addView(etName)
        container.addView(etLink)

        AlertDialog.Builder(this)
            .setTitle("Pin Site to Home")
            .setView(container)
            .setPositiveButton("Pin") { _, _ ->
                val rawUrl = etLink.text.toString().trim()
                if (rawUrl.isNotEmpty()) {
                    val finalUrl = if (rawUrl.startsWith("http")) rawUrl else "https://$rawUrl"
                    val title = etName.text.toString().trim().ifEmpty { Uri.parse(finalUrl).host ?: "Site" }
                    val key = storage.inferIconKey(finalUrl)
                    val list = storage.getShortcuts().toMutableList()
                    list.removeAll { it.url == finalUrl }
                    list.add(0, ShortcutItem(title, finalUrl, key, "#191918"))
                    storage.saveShortcuts(list)
                    rebuildHomeShortcutsGrid()
                    Toast.makeText(this, "Pinned to Home", Toast.LENGTH_SHORT).show()
                }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun scheduleOmniboxSuggestions(query: String) {
        val q = query.trim()
        val currentGen = ++suggestGen
        if (q.isEmpty()) {
            suggestionsScroll.visibility = View.GONE
            return
        }

        val localMatches = mutableListOf<Triple<Int, String, String>>()
        for (b in storage.getBookmarks()) {
            if (b.title.contains(q, true) || b.url.contains(q, true)) {
                localMatches.add(Triple(R.drawable.ic_star_filled, b.title, b.url))
            }
        }
        for (h in storage.getHistory()) {
            if (h.title.contains(q, true) || h.url.contains(q, true)) {
                if (localMatches.none { it.third == h.url }) {
                    localMatches.add(Triple(R.drawable.ic_history, h.title, h.url))
                }
            }
        }

        suggestExecutor.submit {
            val webSuggestions = SmartSearchService.fetchSuggestions(q)
            if (suggestGen != currentGen) return@submit
            runOnUiThread {
                if (!etUrl.hasFocus() || suggestGen != currentGen) return@runOnUiThread
                renderSuggestionsPanel(q, webSuggestions, localMatches.take(3))
            }
        }
    }

    private fun renderSuggestionsPanel(
        query: String,
        webSuggestions: List<String>,
        localMatches: List<Triple<Int, String, String>>
    ) {
        suggestionsPanel.removeAllViews()
        var count = 0

        val isUrl = looksLikeUrl(query)
        val primaryLabel = if (isUrl) "Open $query" else "Search \"$query\" with ${prefs.searchEngine.displayName}"
        suggestionsPanel.addView(
            buildSuggestionRow(
                iconRes = if (isUrl) R.drawable.ic_globe else R.drawable.ic_search,
                title = primaryLabel,
                subtitle = null
            ) {
                hideKeyboard()
                suggestionsScroll.visibility = View.GONE
                submitOmniboxInput(query)
            }
        )
        count++

        for ((iconRes, title, targetUrl) in localMatches) {
            suggestionsPanel.addView(
                buildSuggestionRow(iconRes = iconRes, title = title, subtitle = targetUrl) {
                    hideKeyboard()
                    suggestionsScroll.visibility = View.GONE
                    navigateTo(targetUrl)
                }
            )
            count++
        }

        for (s in webSuggestions) {
            if (s.equals(query, ignoreCase = true)) continue
            suggestionsPanel.addView(
                buildSuggestionRow(iconRes = R.drawable.ic_search, title = s, subtitle = null) {
                    hideKeyboard()
                    suggestionsScroll.visibility = View.GONE
                    performSearch(s)
                }
            )
            count++
            if (count >= 7) break
        }

        suggestionsScroll.visibility = if (count > 0) View.VISIBLE else View.GONE
    }

    private fun buildSuggestionRow(
        iconRes: Int,
        title: String,
        subtitle: String?,
        onClick: () -> Unit
    ): View {
        val darkChrome = currentTab().isIncognito || prefs.appTheme == "dark"
        val primaryText = if (darkChrome) Color.parseColor("#F4F1EA") else Color.parseColor("#191918")
        val secondaryText = if (darkChrome) Color.parseColor("#A19E95") else Color.parseColor("#6B6860")

        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(14), dp(10), dp(14), dp(10))
            val typed = TypedValue()
            context.theme.resolveAttribute(android.R.attr.selectableItemBackground, typed, true)
            setBackgroundResource(typed.resourceId)
            setOnClickListener { onClick() }
        }
        val iv = ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(18), dp(18)).apply { marginEnd = dp(12) }
            setImageResource(iconRes)
            setColorFilter(
                if (iconRes == R.drawable.ic_star_filled) Color.parseColor("#D97757")
                else secondaryText
            )
        }
        val textCol = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        textCol.addView(TextView(this).apply {
            text = title
            textSize = 13.5f
            maxLines = 1
            setTextColor(primaryText)
        })
        if (!subtitle.isNullOrEmpty()) {
            textCol.addView(TextView(this).apply {
                text = subtitle
                textSize = 11.5f
                maxLines = 1
                setTextColor(secondaryText)
            })
        }
        row.addView(iv)
        row.addView(textCol)
        return row
    }

    private fun createNewTab(isIncognito: Boolean) {
        stopLoading()
        val tab = BrowserTab(
            id = nextTabId++,
            title = if (isIncognito) "Private Tab" else "New Tab",
            isIncognito = isIncognito,
            isOnHome = true,
            desktopMode = prefs.desktopModeDefault
        )
        tabs.add(tab)
        activeTabIndex = tabs.lastIndex
        showHomeScreen()
        Toast.makeText(
            this,
            if (isIncognito) "Private tab opened" else "New tab opened",
            Toast.LENGTH_SHORT
        ).show()
    }

    private fun switchToTab(index: Int) {
        if (index !in tabs.indices) return
        stopLoading()
        activeTabIndex = index
        val tab = currentTab()
        if (tab.isOnHome) {
            showHomeScreen()
        } else if (tab.cachedHtml != null) {
            etUrl.setText(if (tab.isSearchSerp) tab.searchQuery else tab.url)
            updateTabUiState()
            nirvanaView.loadHtml(tab.cachedHtml!!, "")
        } else if (tab.url.isNotEmpty()) {
            loadUrlInternal(tab.url)
        } else {
            showHomeScreen()
        }
    }

    private fun closeTabAt(index: Int) {
        if (index !in tabs.indices) return
        val closing = tabs[index]
        if (closing.isIncognito && tabs.count { it.isIncognito } <= 1) {
            nirvanaView.engine.clearCookies()
        }
        if (tabs.size <= 1) {
            tabs[0] = BrowserTab(
                id = nextTabId++,
                title = "New Tab",
                isOnHome = true,
                desktopMode = prefs.desktopModeDefault
            )
            activeTabIndex = 0
            showHomeScreen()
            return
        }
        tabs.removeAt(index)
        if (activeTabIndex == index) {
            activeTabIndex = activeTabIndex.coerceAtMost(tabs.lastIndex)
            switchToTab(activeTabIndex)
        } else {
            if (activeTabIndex > index) activeTabIndex--
            updateTabUiState()
        }
    }

    private fun closeActiveTab() {
        closeTabAt(activeTabIndex)
    }

    private data class SheetPalette(
        val surfaceBg: Int,
        val controlBg: Int,
        val borderColor: Int,
        val activeRowBg: Int,
        val primaryText: Int,
        val secondaryText: Int
    )

    private fun currentSheetPalette(): SheetPalette {
        val tab = currentTab()
        val darkChrome = tab.isIncognito || prefs.appTheme == "dark"
        return SheetPalette(
            surfaceBg = when {
                tab.isIncognito -> Color.parseColor("#1D1B24")
                prefs.appTheme == "dark" -> Color.parseColor("#1C1B19")
                else -> Color.parseColor("#FFFFFF")
            },
            controlBg = when {
                tab.isIncognito -> Color.parseColor("#2A2835")
                prefs.appTheme == "dark" -> Color.parseColor("#272623")
                else -> Color.parseColor("#F2EFE9")
            },
            borderColor = when {
                tab.isIncognito -> Color.parseColor("#353242")
                prefs.appTheme == "dark" -> Color.parseColor("#353430")
                else -> Color.parseColor("#E5E0D5")
            },
            activeRowBg = when {
                darkChrome -> Color.parseColor("#2E2623")
                else -> Color.parseColor("#FBF1EC")
            },
            primaryText = if (darkChrome) Color.parseColor("#F4F1EA") else Color.parseColor("#191918"),
            secondaryText = if (darkChrome) Color.parseColor("#A19E95") else Color.parseColor("#6B6860")
        )
    }

    private fun prepareThemedBottomSheet(dialog: BottomSheetDialog, view: View, pal: SheetPalette) {
        view.background = createTopRoundedBox(pal.surfaceBg, 22)
        if (view is LinearLayout && view.childCount > 0) {
            view.getChildAt(0).setBackgroundColor(pal.borderColor)
            for (i in 0 until view.childCount) {
                val c = view.getChildAt(i)
                if (c !is android.view.ViewGroup && c.layoutParams?.height in 1..4 && i > 0) {
                    c.setBackgroundColor(pal.borderColor)
                }
            }
        }
        dialog.setOnShowListener {
            dialog.findViewById<View>(com.google.android.material.R.id.design_bottom_sheet)
                ?.setBackgroundColor(Color.TRANSPARENT)
        }
    }

    private fun showTabsSheet() {
        val pal = currentSheetPalette()
        val dialog = BottomSheetDialog(this)
        val view = LayoutInflater.from(this).inflate(R.layout.sheet_list, null)
        prepareThemedBottomSheet(dialog, view, pal)
        dialog.setContentView(view)

        val ivHeader = view.findViewById<ImageView>(R.id.ivSheetHeaderIcon)
        ivHeader.setImageResource(R.drawable.ic_tabs)
        ivHeader.setColorFilter(pal.primaryText)
        val tvTitle = view.findViewById<TextView>(R.id.tvSheetTitle)
        tvTitle.setTextColor(pal.primaryText)
        val btnPrivate = view.findViewById<TextView>(R.id.btnSheetActionSecondary)
        val btnNewTab = view.findViewById<TextView>(R.id.btnSheetActionPrimary)
        val container = view.findViewById<LinearLayout>(R.id.sheetListContainer)

        btnPrivate.visibility = View.VISIBLE
        btnPrivate.text = "+ Private"
        btnPrivate.setTextColor(pal.primaryText)
        btnPrivate.background = createStrokeBox(pal.controlBg, pal.borderColor, 14)
        btnPrivate.setOnClickListener {
            dialog.dismiss()
            createNewTab(true)
        }

        btnNewTab.visibility = View.VISIBLE
        btnNewTab.text = "+ New Tab"
        btnNewTab.setTextColor(Color.WHITE)
        btnNewTab.background = createStrokeBox(Color.parseColor("#D97757"), Color.parseColor("#D97757"), 14)
        btnNewTab.setOnClickListener {
            dialog.dismiss()
            createNewTab(false)
        }

        fun renderTabRows() {
            tvTitle.text = "Tabs (${tabs.size})"
            container.removeAllViews()
            tabs.forEachIndexed { idx, tab ->
                val isActive = idx == activeTabIndex
                val row = LinearLayout(this).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(dp(16), dp(12), dp(10), dp(12))
                    if (isActive) {
                        setBackgroundColor(pal.activeRowBg)
                    }
                    setOnClickListener {
                        dialog.dismiss()
                        switchToTab(idx)
                    }
                }
                val icon = ImageView(this).apply {
                    layoutParams = LinearLayout.LayoutParams(dp(18), dp(18)).apply { marginEnd = dp(12) }
                    setImageResource(if (tab.isIncognito) R.drawable.ic_incognito else R.drawable.ic_globe)
                    setColorFilter(
                        if (isActive) Color.parseColor("#D97757") else pal.secondaryText
                    )
                }
                val col = LinearLayout(this).apply {
                    orientation = LinearLayout.VERTICAL
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                }
                col.addView(TextView(this).apply {
                    text = tab.title.ifBlank { if (tab.isOnHome) "New Tab" else tab.url }
                    textSize = 14f
                    maxLines = 1
                    typeface = if (isActive) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
                    setTextColor(pal.primaryText)
                })
                col.addView(TextView(this).apply {
                    text = when {
                        tab.isOnHome && tab.isIncognito -> "Private Start Screen"
                        tab.isOnHome -> "Start Screen"
                        else -> tab.url
                    }
                    textSize = 11.5f
                    maxLines = 1
                    setTextColor(pal.secondaryText)
                })
                val closeBtn = ImageButton(this).apply {
                    layoutParams = LinearLayout.LayoutParams(dp(34), dp(34))
                    setImageResource(R.drawable.ic_close)
                    setPadding(dp(8), dp(8), dp(8), dp(8))
                    setColorFilter(pal.secondaryText)
                    val typed = TypedValue()
                    context.theme.resolveAttribute(android.R.attr.selectableItemBackgroundBorderless, typed, true)
                    setBackgroundResource(typed.resourceId)
                    setOnClickListener {
                        closeTabAt(idx)
                        if (tabs.size <= 1 && tabs[0].isOnHome) {
                            dialog.dismiss()
                        } else {
                            renderTabRows()
                        }
                    }
                }
                row.addView(icon)
                row.addView(col)
                row.addView(closeBtn)
                container.addView(row)
            }
        }
        renderTabRows()
        dialog.show()
    }

    private fun showOverflowSheet() {
        val tab = currentTab()
        val pal = currentSheetPalette()
        val dialog = BottomSheetDialog(this)
        val view = LayoutInflater.from(this).inflate(R.layout.sheet_overflow_menu, null)
        prepareThemedBottomSheet(dialog, view, pal)
        dialog.setContentView(view)

        val btnFwd = view.findViewById<ImageButton>(R.id.sheetBtnForward)
        btnFwd.setColorFilter(pal.primaryText)
        btnFwd.isEnabled = tab.forwardStack.isNotEmpty()
        btnFwd.alpha = if (btnFwd.isEnabled) 1f else 0.35f
        btnFwd.setOnClickListener {
            dialog.dismiss()
            navigateForward()
        }

        val isBookmarked = !tab.isOnHome && storage.isBookmarked(tab.url)
        val btnStar = view.findViewById<ImageButton>(R.id.sheetBtnBookmark)
        btnStar.setImageResource(if (isBookmarked) R.drawable.ic_star_filled else R.drawable.ic_star_outline)
        btnStar.setColorFilter(if (isBookmarked) Color.parseColor("#D97757") else pal.primaryText)
        btnStar.setOnClickListener {
            dialog.dismiss()
            if (tab.isOnHome) showBookmarksSheet() else toggleCurrentBookmark()
        }

        val btnShare = view.findViewById<ImageButton>(R.id.sheetBtnShare)
        btnShare.setColorFilter(pal.primaryText)
        btnShare.setOnClickListener {
            dialog.dismiss()
            shareCurrentUrl()
        }

        val btnPin = view.findViewById<ImageButton>(R.id.sheetBtnPinHome)
        btnPin.setColorFilter(pal.primaryText)
        btnPin.setOnClickListener {
            dialog.dismiss()
            if (tab.isOnHome || tab.url.isEmpty()) {
                showAddShortcutDialog()
            } else {
                showAddShortcutDialog(tab.title, tab.url)
            }
        }

        val btnRel = view.findViewById<ImageButton>(R.id.sheetBtnReload)
        btnRel.setColorFilter(pal.primaryText)
        btnRel.setImageResource(if (isLoading) R.drawable.ic_stop else R.drawable.ic_refresh)
        btnRel.setOnClickListener {
            dialog.dismiss()
            if (isLoading) stopLoading() else reloadCurrentTab()
        }

        val container = view.findViewById<LinearLayout>(R.id.menuItemsContainer)
        fun addMenuRow(iconRes: Int, title: String, statusTag: String? = null, onClick: () -> Unit) {
            val row = LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(20), dp(13), dp(20), dp(13))
                val typed = TypedValue()
                context.theme.resolveAttribute(android.R.attr.selectableItemBackground, typed, true)
                setBackgroundResource(typed.resourceId)
                setOnClickListener {
                    dialog.dismiss()
                    onClick()
                }
            }
            row.addView(ImageView(this).apply {
                layoutParams = LinearLayout.LayoutParams(dp(20), dp(20)).apply { marginEnd = dp(16) }
                setImageResource(iconRes)
                setColorFilter(pal.primaryText)
            })
            row.addView(TextView(this).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                text = title
                textSize = 14.5f
                setTextColor(pal.primaryText)
            })
            if (statusTag != null) {
                row.addView(TextView(this).apply {
                    text = statusTag
                    textSize = 11.5f
                    typeface = Typeface.DEFAULT_BOLD
                    val isOn = statusTag == "ON" || statusTag == "Active"
                    setTextColor(if (isOn) Color.WHITE else pal.secondaryText)
                    background = createStrokeBox(
                        if (isOn) Color.parseColor("#D97757") else pal.controlBg,
                        if (isOn) Color.parseColor("#D97757") else pal.borderColor,
                        12
                    )
                    setPadding(dp(9), dp(3), dp(9), dp(3))
                })
            }
            container.addView(row)
        }

        addMenuRow(R.drawable.ic_add, "New Tab") { createNewTab(false) }
        addMenuRow(R.drawable.ic_incognito, "New Private Tab") { createNewTab(true) }
        addMenuRow(R.drawable.ic_bookmarks, "Bookmarks") { showBookmarksSheet() }
        addMenuRow(R.drawable.ic_history, "History") { showHistorySheet() }
        addMenuRow(R.drawable.ic_search, "Find in Page") { openFindInPageBar() }
        addMenuRow(
            R.drawable.ic_reader,
            if (tab.readerModeActive) "Exit Reader View" else "Reader View",
            if (tab.readerModeActive) "Active" else null
        ) { toggleReaderMode() }
        addMenuRow(
            R.drawable.ic_desktop,
            "Desktop Site",
            if (tab.desktopMode) "ON" else "OFF"
        ) {
            tab.desktopMode = !tab.desktopMode
            Toast.makeText(this, if (tab.desktopMode) "Desktop mode enabled" else "Mobile mode enabled", Toast.LENGTH_SHORT).show()
            reloadCurrentTab()
        }
        addMenuRow(
            R.drawable.ic_dark_mode,
            "Dark Web Contents",
            if (prefs.forceDarkWeb) "ON" else "OFF"
        ) {
            prefs.forceDarkWeb = !prefs.forceDarkWeb
            applyPrefsToUiAndEngine()
            Toast.makeText(this, if (prefs.forceDarkWeb) "Web dark mode enabled" else "Web dark mode off", Toast.LENGTH_SHORT).show()
        }
        addMenuRow(
            R.drawable.ic_shield,
            "Protection Shield",
            "${nirvanaView.engine.getBlockedRequestsCount()} blocked"
        ) { showShieldDialog() }
        addMenuRow(R.drawable.ic_settings, "Settings") {
            settingsLauncher.launch(Intent(this, SettingsActivity::class.java))
        }

        dialog.show()
    }

    private fun toggleCurrentBookmark() {
        val tab = currentTab()
        if (tab.isOnHome || tab.url.isEmpty()) {
            showBookmarksSheet()
            return
        }
        val added = storage.toggleBookmark(tab.title, tab.url)
        updateTabUiState()
        Toast.makeText(this, if (added) "Bookmark saved" else "Bookmark removed", Toast.LENGTH_SHORT).show()
    }

    private fun showBookmarksSheet() {
        val pal = currentSheetPalette()
        val dialog = BottomSheetDialog(this)
        val view = LayoutInflater.from(this).inflate(R.layout.sheet_list, null)
        prepareThemedBottomSheet(dialog, view, pal)
        dialog.setContentView(view)

        val ivHeader = view.findViewById<ImageView>(R.id.ivSheetHeaderIcon)
        ivHeader.setImageResource(R.drawable.ic_bookmarks)
        ivHeader.setColorFilter(pal.primaryText)
        val tvTitle = view.findViewById<TextView>(R.id.tvSheetTitle)
        tvTitle.setTextColor(pal.primaryText)
        val btnAddCurrent = view.findViewById<TextView>(R.id.btnSheetActionPrimary)
        val container = view.findViewById<LinearLayout>(R.id.sheetListContainer)

        val tab = currentTab()
        if (!tab.isOnHome && tab.url.isNotEmpty() && !storage.isBookmarked(tab.url)) {
            btnAddCurrent.visibility = View.VISIBLE
            btnAddCurrent.text = "+ Bookmark Page"
            btnAddCurrent.setTextColor(Color.WHITE)
            btnAddCurrent.background = createStrokeBox(Color.parseColor("#D97757"), Color.parseColor("#D97757"), 14)
        }

        fun renderList() {
            val list = storage.getBookmarks()
            tvTitle.text = "Bookmarks (${list.size})"
            container.removeAllViews()
            if (list.isEmpty()) {
                container.addView(TextView(this).apply {
                    text = "No bookmarks saved yet."
                    setPadding(dp(20), dp(24), dp(20), dp(24))
                    setTextColor(pal.secondaryText)
                })
                return
            }
            for (item in list) {
                container.addView(buildLinkRow(
                    iconRes = R.drawable.ic_star_filled,
                    iconColor = Color.parseColor("#D97757"),
                    title = item.title,
                    subtitle = item.url,
                    pal = pal,
                    onClick = {
                        dialog.dismiss()
                        navigateTo(item.url)
                    },
                    onDelete = {
                        val updated = storage.getBookmarks().filterNot { it.url == item.url }
                        storage.saveBookmarks(updated)
                        updateTabUiState()
                        renderList()
                    }
                ))
            }
        }

        btnAddCurrent.setOnClickListener {
            storage.toggleBookmark(tab.title, tab.url)
            btnAddCurrent.visibility = View.GONE
            updateTabUiState()
            renderList()
        }

        renderList()
        dialog.show()
    }

    private fun showHistorySheet() {
        val pal = currentSheetPalette()
        val dialog = BottomSheetDialog(this)
        val view = LayoutInflater.from(this).inflate(R.layout.sheet_list, null)
        prepareThemedBottomSheet(dialog, view, pal)
        dialog.setContentView(view)

        val ivHeader = view.findViewById<ImageView>(R.id.ivSheetHeaderIcon)
        ivHeader.setImageResource(R.drawable.ic_history)
        ivHeader.setColorFilter(pal.primaryText)
        val tvTitle = view.findViewById<TextView>(R.id.tvSheetTitle)
        tvTitle.setTextColor(pal.primaryText)
        val btnClear = view.findViewById<TextView>(R.id.btnSheetActionSecondary)
        val container = view.findViewById<LinearLayout>(R.id.sheetListContainer)

        btnClear.visibility = View.VISIBLE
        btnClear.text = "Clear History"
        btnClear.setTextColor(pal.primaryText)
        btnClear.background = createStrokeBox(pal.controlBg, pal.borderColor, 14)
        btnClear.setOnClickListener {
            storage.clearHistory()
            dialog.dismiss()
            Toast.makeText(this, "History cleared", Toast.LENGTH_SHORT).show()
        }

        val list = storage.getHistory()
        tvTitle.text = "History (${list.size})"
        if (list.isEmpty()) {
            btnClear.visibility = View.GONE
            container.addView(TextView(this).apply {
                text = "No browsing history yet."
                setPadding(dp(20), dp(24), dp(20), dp(24))
                setTextColor(pal.secondaryText)
            })
        } else {
            for (item in list.take(40)) {
                container.addView(buildLinkRow(
                    iconRes = R.drawable.ic_history,
                    iconColor = pal.secondaryText,
                    title = item.title,
                    subtitle = item.url,
                    pal = pal,
                    onClick = {
                        dialog.dismiss()
                        navigateTo(item.url)
                    },
                    onDelete = null
                ))
            }
        }
        dialog.show()
    }

    private fun buildLinkRow(
        iconRes: Int,
        iconColor: Int,
        title: String,
        subtitle: String,
        pal: SheetPalette,
        onClick: () -> Unit,
        onDelete: (() -> Unit)?
    ): View {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(16), dp(11), dp(10), dp(11))
            val typed = TypedValue()
            context.theme.resolveAttribute(android.R.attr.selectableItemBackground, typed, true)
            setBackgroundResource(typed.resourceId)
            setOnClickListener { onClick() }
        }
        row.addView(ImageView(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(18), dp(18)).apply { marginEnd = dp(12) }
            setImageResource(iconRes)
            setColorFilter(iconColor)
        })
        val col = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        col.addView(TextView(this).apply {
            text = title
            textSize = 14f
            maxLines = 1
            setTextColor(pal.primaryText)
        })
        col.addView(TextView(this).apply {
            text = subtitle
            textSize = 11.5f
            maxLines = 1
            setTextColor(pal.secondaryText)
        })
        row.addView(col)
        if (onDelete != null) {
            row.addView(ImageButton(this).apply {
                layoutParams = LinearLayout.LayoutParams(dp(34), dp(34))
                setImageResource(R.drawable.ic_trash)
                setPadding(dp(8), dp(8), dp(8), dp(8))
                val typed = TypedValue()
                context.theme.resolveAttribute(android.R.attr.selectableItemBackgroundBorderless, typed, true)
                setBackgroundResource(typed.resourceId)
                setOnClickListener { onDelete() }
            })
        }
        return row
    }

    private fun showSearchEngineSheet() {
        val pal = currentSheetPalette()
        val dialog = BottomSheetDialog(this)
        val view = LayoutInflater.from(this).inflate(R.layout.sheet_list, null)
        prepareThemedBottomSheet(dialog, view, pal)
        dialog.setContentView(view)

        val ivHeader = view.findViewById<ImageView>(R.id.ivSheetHeaderIcon)
        ivHeader.setImageResource(R.drawable.ic_search)
        ivHeader.setColorFilter(pal.primaryText)
        val tvTitle = view.findViewById<TextView>(R.id.tvSheetTitle)
        tvTitle.text = "Search Engine"
        tvTitle.setTextColor(pal.primaryText)
        val container = view.findViewById<LinearLayout>(R.id.sheetListContainer)
        val active = prefs.searchEngine

        for (engine in SearchEngine.values()) {
            val isSel = engine == active
            val row = LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(18), dp(13), dp(18), dp(13))
                if (isSel) setBackgroundColor(pal.activeRowBg)
                setOnClickListener {
                    selectSearchEngine(engine)
                    dialog.dismiss()
                }
            }
            val badge = TextView(this).apply {
                layoutParams = LinearLayout.LayoutParams(dp(26), dp(26)).apply { marginEnd = dp(14) }
                gravity = Gravity.CENTER
                text = engine.shortCode
                textSize = 11.5f
                typeface = Typeface.DEFAULT_BOLD
                setTextColor(Color.WHITE)
                background = createCircleBadgeDrawable(
                    if (isSel) Color.parseColor("#D97757") else pal.secondaryText
                )
            }
            val label = TextView(this).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                text = engine.displayName
                textSize = 14.5f
                typeface = if (isSel) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
                setTextColor(pal.primaryText)
            }
            row.addView(badge)
            row.addView(label)
            if (isSel) {
                row.addView(ImageView(this).apply {
                    layoutParams = LinearLayout.LayoutParams(dp(18), dp(18))
                    setImageResource(R.drawable.ic_check)
                })
            }
            container.addView(row)
        }
        dialog.show()
    }

    private fun openFindInPageBar() {
        if (currentTab().isOnHome) {
            Toast.makeText(this, "Open a web page first", Toast.LENGTH_SHORT).show()
            return
        }
        findInPageBar.visibility = View.VISIBLE
        etFindQuery.requestFocus()
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.showSoftInput(etFindQuery, InputMethodManager.SHOW_IMPLICIT)
    }

    private fun runFindInPage() {
        val q = etFindQuery.text.toString().trim()
        if (q.isEmpty()) {
            lastFindTotal = 0
            tvFindCount.text = "0/0"
            return
        }
        val total = nirvanaView.engine.findInPage(q, findMatchIndex)
        lastFindTotal = total
        if (total == 0) {
            tvFindCount.text = "0/0"
        } else {
            val dispIdx = (findMatchIndex % total) + 1
            tvFindCount.text = "$dispIdx/$total"
            nirvanaView.requestRender()
        }
    }

    private fun toggleReaderMode() {
        val tab = currentTab()
        if (tab.isOnHome) {
            Toast.makeText(this, "Open an article first", Toast.LENGTH_SHORT).show()
            return
        }
        if (tab.readerModeActive) {
            tab.readerModeActive = false
            reloadCurrentTab()
        } else {
            val readerHtml = nirvanaView.engine.extractReaderHtml()
            if (readerHtml == null) {
                Toast.makeText(this, "Reader view unavailable for this page", Toast.LENGTH_SHORT).show()
            } else {
                tab.readerModeActive = true
                nirvanaView.loadHtml(readerHtml, "")
                Toast.makeText(this, "Reader view enabled", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun shareCurrentUrl() {
        val tab = currentTab()
        if (tab.isOnHome || tab.url.isEmpty()) {
            Toast.makeText(this, "Open a webpage to share", Toast.LENGTH_SHORT).show()
            return
        }
        val cm = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        cm.setPrimaryClip(ClipData.newPlainText("URL", tab.url))
        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_SUBJECT, tab.title)
            putExtra(Intent.EXTRA_TEXT, tab.url)
        }
        startActivity(Intent.createChooser(shareIntent, "Share link (copied to clipboard)"))
    }

    private fun showSiteSecurityDialog() {
        val tab = currentTab()
        val host = Uri.parse(tab.url).host ?: tab.url
        val isHttps = tab.url.startsWith("https://")
        val metrics = nirvanaView.engine.getFrameMetrics()
        val msg = StringBuilder()
        msg.append(if (isHttps) "Connection: Encrypted (HTTPS / TLS)\n" else "Connection: Unencrypted (HTTP)\n")
        msg.append("Host: $host\n")
        msg.append("Viewport: ${if (tab.desktopMode) "Desktop" else "Mobile"} | JS: ${if (prefs.jsEnabled) "ON" else "OFF"}\n\n")
        msg.append("Engine Frame Telemetry:\n")
        msg.append("Parse: ${"%.1f".format(metrics.parseMs)} ms | Style: ${"%.1f".format(metrics.styleMs)} ms\n")
        msg.append("Layout: ${"%.1f".format(metrics.layoutMs)} ms | Raster: ${"%.1f".format(metrics.rasterMs)} ms")

        AlertDialog.Builder(this)
            .setTitle(tab.title.ifBlank { host })
            .setMessage(msg.toString())
            .setPositiveButton("Copy URL") { _, _ ->
                val cm = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                cm.setPrimaryClip(ClipData.newPlainText("URL", tab.url))
                Toast.makeText(this, "URL copied", Toast.LENGTH_SHORT).show()
            }
            .setNeutralButton("Protection") { _, _ -> showShieldDialog() }
            .setNegativeButton("Close", null)
            .show()
    }

    private fun showShieldDialog() {
        val blockedCount = nirvanaView.engine.getBlockedRequestsCount()
        val isEnabled = nirvanaView.engine.isAdBlockEnabled()
        val statusText = if (isEnabled) "Active (Network + Cosmetic Blocker)" else "Paused"

        AlertDialog.Builder(this)
            .setTitle(getString(R.string.shield_title))
            .setMessage(
                "Trackers & Ads Blocked: $blockedCount\n" +
                    "Status: $statusText\n\n" +
                    "Nirvana intercepts ad networks, analytics beacons, and cosmetic banners directly inside the pure-Rust HTTP stack."
            )
            .setPositiveButton(if (isEnabled) "Pause Shield" else "Enable Shield") { _, _ ->
                val next = !isEnabled
                prefs.adBlockEnabled = next
                nirvanaView.engine.setAdBlockEnabled(next)
                updateShieldBadge()
            }
            .setNeutralButton("Reset Counter") { _, _ ->
                nirvanaView.engine.resetBlockedCount()
                updateShieldBadge()
            }
            .setNegativeButton("Close", null)
            .show()
    }

    private fun buildErrorPageHtml(url: String, errorMsg: String): String {
        val safeUrl = url.replace("<", "&lt;").replace(">", "&gt;")
        val safeErr = errorMsg.replace("<", "&lt;").replace(">", "&gt;")
        return """
            <!DOCTYPE html>
            <html>
            <head><title>Unable to load page</title></head>
            <body style="background-color:#FAF9F5; padding:24px; color:#191918;">
                <h2 style="color:#B83A3A; font-size:21px; margin-bottom:8px;">Unable to load page</h2>
                <p style="color:#6B6860; font-size:14px;">$safeUrl</p>
                <div style="background-color:#FFFFFF; border:1px solid #E5E0D5; padding:14px; border-radius:12px; margin-top:14px;">
                    <b>Detail:</b> $safeErr
                </div>
                <p style="margin-top:16px;"><a href="$safeUrl" style="color:#D97757; font-weight:bold;">Try loading again</a></p>
            </body>
            </html>
        """.trimIndent()
    }

    private fun createCircleBadgeDrawable(color: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(color)
        }
    }

    private fun createStrokeBox(fillColor: Int, strokeColor: Int, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = (radiusDp * resources.displayMetrics.density)
            setColor(fillColor)
            setStroke(dp(1), strokeColor)
        }
    }

    private fun createTopRoundedBox(fillColor: Int, radiusDp: Int): GradientDrawable {
        val r = radiusDp * resources.displayMetrics.density
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadii = floatArrayOf(r, r, r, r, 0f, 0f, 0f, 0f)
            setColor(fillColor)
        }
    }

    private fun createOutlineStrokeBox(strokeColor: Int, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = (radiusDp * resources.displayMetrics.density)
            setColor(Color.TRANSPARENT)
            setStroke((1.6f * resources.displayMetrics.density).toInt().coerceAtLeast(1), strokeColor)
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private fun handleIncomingIntent(intent: Intent) {
        if (intent.action == Intent.ACTION_VIEW) {
            intent.data?.let { navigateTo(it.toString()) }
        }
    }

    private fun hideKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(etUrl.windowToken, 0)
    }
}
