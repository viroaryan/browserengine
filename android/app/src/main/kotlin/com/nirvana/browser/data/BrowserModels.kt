package com.nirvana.browser.data

import android.content.Context
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.util.regex.Pattern

/**
 * Supported search providers with badge branding, query endpoints, and home URLs.
 */
enum class SearchEngine(
    val displayName: String,
    val searchUrl: String,
    val homeUrl: String,
    val shortCode: String,
    val badgeColorHex: String
) {
    GOOGLE("Google", "https://www.google.com/search?gbv=1&q=", "https://www.google.com", "G", "#191918"),
    DUCKDUCKGO("DuckDuckGo", "https://html.duckduckgo.com/html/?q=", "https://html.duckduckgo.com/html/", "D", "#D97757"),
    BING("Bing", "https://www.bing.com/search?q=", "https://www.bing.com", "B", "#2F6F4E"),
    BRAVE("Brave", "https://search.brave.com/search?q=", "https://search.brave.com", "Br", "#C26142"),
    ECOSIA("Ecosia", "https://www.ecosia.org/search?q=", "https://www.ecosia.org", "E", "#3B7A57")
}

/**
 * Pure omnibox input classifier and URL formatter shared by UI and unit tests.
 */
object OmniboxRouter {
    fun looksLikeUrl(input: String): Boolean {
        val trimmed = input.trim()
        if (trimmed.isEmpty() || trimmed.contains(" ")) return false
        val lower = trimmed.lowercase()
        if (lower.startsWith("http://") ||
            lower.startsWith("https://") ||
            lower.startsWith("data:") ||
            lower.startsWith("file://")
        ) {
            return true
        }
        if (lower == "localhost" || lower.startsWith("localhost:") || lower.startsWith("localhost/")) {
            return true
        }
        val dotIdx = trimmed.indexOf('.')
        return dotIdx > 0 && dotIdx < trimmed.length - 1
    }

    fun normalizeUrl(input: String): String {
        val trimmed = input.trim()
        val lower = trimmed.lowercase()
        return when {
            lower.startsWith("http://") ||
                lower.startsWith("https://") ||
                lower.startsWith("data:") ||
                lower.startsWith("file://") -> trimmed
            lower == "localhost" || lower.startsWith("localhost:") || lower.startsWith("localhost/") ->
                "http://$trimmed"
            else -> "https://$trimmed"
        }
    }
}

data class HistoryStep(
    val url: String,
    val isSearchSerp: Boolean = false,
    val searchQuery: String = "",
    val cachedHtml: String? = null
)

/**
 * Represents a single browser tab's navigation state and metadata.
 */
data class BrowserTab(
    val id: Long,
    var title: String = "New Tab",
    var url: String = "",
    var isIncognito: Boolean = false,
    var isOnHome: Boolean = true,
    var isSearchSerp: Boolean = false,
    var searchQuery: String = "",
    var cachedHtml: String? = null,
    val backStack: MutableList<HistoryStep> = mutableListOf(),
    val forwardStack: MutableList<HistoryStep> = mutableListOf(),
    var readerModeActive: Boolean = false,
    var desktopMode: Boolean = false
)

data class BookmarkItem(val title: String, val url: String, val createdAt: Long = System.currentTimeMillis())

data class HistoryItem(val title: String, val url: String, val visitedAt: Long = System.currentTimeMillis())

data class ShortcutItem(
    val title: String,
    val url: String,
    val iconKey: String = "globe",
    val colorHex: String = "#191918"
)

/**
 * Typed SharedPreferences facade for Chrome/Firefox-grade browser settings.
 */
class BrowserPrefs(context: Context) {
    private val prefs = context.getSharedPreferences("nirvana_prefs", Context.MODE_PRIVATE)

    var searchEngine: SearchEngine
        get() {
            val raw = prefs.getString("search_engine", SearchEngine.GOOGLE.name)
            return try {
                SearchEngine.valueOf(raw ?: SearchEngine.GOOGLE.name)
            } catch (_: Exception) {
                SearchEngine.GOOGLE
            }
        }
        set(value) = prefs.edit().putString("search_engine", value.name).apply()

    var smartSearchFallback: Boolean
        get() = prefs.getBoolean("smart_search_fallback", true)
        set(value) = prefs.edit().putBoolean("smart_search_fallback", value).apply()

    var appTheme: String
        get() = prefs.getString("app_theme", "paper") ?: "paper"
        set(value) = prefs.edit().putString("app_theme", value).apply()

    var forceDarkWeb: Boolean
        get() = prefs.getBoolean("force_dark_web", false)
        set(value) = prefs.edit().putBoolean("force_dark_web", value).apply()

    var textZoomPercent: Int
        get() = prefs.getInt("text_zoom_percent", 100).coerceIn(60, 160)
        set(value) = prefs.edit().putInt("text_zoom_percent", value.coerceIn(60, 160)).apply()

    var adBlockEnabled: Boolean
        get() = prefs.getBoolean("adblock_enabled", true)
        set(value) = prefs.edit().putBoolean("adblock_enabled", value).apply()

    var httpsOnly: Boolean
        get() = prefs.getBoolean("https_only", false)
        set(value) = prefs.edit().putBoolean("https_only", value).apply()

    var doNotTrack: Boolean
        get() = prefs.getBoolean("do_not_track", true)
        set(value) = prefs.edit().putBoolean("do_not_track", value).apply()

    var jsEnabled: Boolean
        get() = prefs.getBoolean("js_enabled", true)
        set(value) = prefs.edit().putBoolean("js_enabled", value).apply()

    var desktopModeDefault: Boolean
        get() = prefs.getBoolean("desktop_mode_default", false)
        set(value) = prefs.edit().putBoolean("desktop_mode_default", value).apply()

    var showShortcuts: Boolean
        get() = prefs.getBoolean("show_shortcuts", true)
        set(value) = prefs.edit().putBoolean("show_shortcuts", value).apply()

    var showBottomBar: Boolean
        get() = prefs.getBoolean("show_bottom_bar", true)
        set(value) = prefs.edit().putBoolean("show_bottom_bar", value).apply()

    fun getCustomAdRules(): List<String> {
        val raw = prefs.getString("custom_ad_rules", "[]") ?: "[]"
        val out = mutableListOf<String>()
        try {
            val arr = JSONArray(raw)
            for (i in 0 until arr.length()) {
                out.add(arr.getString(i))
            }
        } catch (_: Exception) {}
        return out
    }

    fun addCustomAdRule(rule: String) {
        val cleaned = rule.trim()
        if (cleaned.isEmpty()) return
        val existing = getCustomAdRules().toMutableList()
        if (!existing.contains(cleaned)) {
            existing.add(cleaned)
            val arr = JSONArray()
            existing.forEach { arr.put(it) }
            prefs.edit().putString("custom_ad_rules", arr.toString()).apply()
        }
    }
}

/**
 * Persistent JSON store for Bookmarks, History, and Customizable Home Speed Dial Shortcuts.
 */
class BrowserStorage(context: Context) {
    private val prefs = context.getSharedPreferences("nirvana_storage", Context.MODE_PRIVATE)

    fun getBookmarks(): MutableList<BookmarkItem> {
        val raw = prefs.getString("bookmarks", null)
        if (raw == null) {
            val defaults = mutableListOf(
                BookmarkItem("Wikipedia", "https://en.m.wikipedia.org"),
                BookmarkItem("Hacker News", "https://news.ycombinator.com"),
                BookmarkItem("GitHub", "https://github.com"),
                BookmarkItem("DuckDuckGo", "https://html.duckduckgo.com/html/"),
                BookmarkItem("Example Domain", "https://example.com")
            )
            saveBookmarks(defaults)
            return defaults
        }
        val out = mutableListOf<BookmarkItem>()
        try {
            val arr = JSONArray(raw)
            for (i in 0 until arr.length()) {
                val obj = arr.getJSONObject(i)
                out.add(
                    BookmarkItem(
                        obj.optString("title", "Bookmark"),
                        obj.optString("url", ""),
                        obj.optLong("createdAt", 0L)
                    )
                )
            }
        } catch (_: Exception) {}
        return out
    }

    fun saveBookmarks(list: List<BookmarkItem>) {
        val arr = JSONArray()
        for (item in list) {
            arr.put(JSONObject().apply {
                put("title", item.title)
                put("url", item.url)
                put("createdAt", item.createdAt)
            })
        }
        prefs.edit().putString("bookmarks", arr.toString()).apply()
    }

    fun isBookmarked(url: String): Boolean {
        if (url.isEmpty()) return false
        return getBookmarks().any { it.url.equals(url, ignoreCase = true) }
    }

    fun toggleBookmark(title: String, url: String): Boolean {
        val list = getBookmarks()
        val existingIdx = list.indexOfFirst { it.url.equals(url, ignoreCase = true) }
        return if (existingIdx >= 0) {
            list.removeAt(existingIdx)
            saveBookmarks(list)
            false
        } else {
            val cleanTitle = title.ifBlank { Uri.parse(url).host ?: url }
            list.add(0, BookmarkItem(cleanTitle, url))
            saveBookmarks(list)
            true
        }
    }

    fun getHistory(): MutableList<HistoryItem> {
        val raw = prefs.getString("history", "[]") ?: "[]"
        val out = mutableListOf<HistoryItem>()
        try {
            val arr = JSONArray(raw)
            for (i in 0 until arr.length()) {
                val obj = arr.getJSONObject(i)
                out.add(
                    HistoryItem(
                        obj.optString("title", "Page"),
                        obj.optString("url", ""),
                        obj.optLong("visitedAt", 0L)
                    )
                )
            }
        } catch (_: Exception) {}
        return out
    }

    fun recordHistory(title: String, url: String) {
        if (url.isEmpty() || url.startsWith("nirvana://") || url.startsWith("about:")) return
        val list = getHistory()
        list.removeAll { it.url == url }
        val displayTitle = title.ifBlank { Uri.parse(url).host ?: url }
        list.add(0, HistoryItem(displayTitle, url))
        val trimmed = if (list.size > 300) list.subList(0, 300) else list
        val arr = JSONArray()
        for (item in trimmed) {
            arr.put(JSONObject().apply {
                put("title", item.title)
                put("url", item.url)
                put("visitedAt", item.visitedAt)
            })
        }
        prefs.edit().putString("history", arr.toString()).apply()
    }

    fun clearHistory() {
        prefs.edit().putString("history", "[]").apply()
    }

    fun getShortcuts(): MutableList<ShortcutItem> {
        val raw = prefs.getString("shortcuts_v2", null)
        if (raw == null) {
            val defaults = defaultShortcuts()
            saveShortcuts(defaults)
            return defaults
        }
        val out = mutableListOf<ShortcutItem>()
        try {
            val arr = JSONArray(raw)
            for (i in 0 until arr.length()) {
                val obj = arr.getJSONObject(i)
                val url = obj.optString("url", "https://example.com")
                out.add(
                    ShortcutItem(
                        obj.optString("title", "Site"),
                        url,
                        obj.optString("iconKey", inferIconKey(url)),
                        obj.optString("colorHex", "#191918")
                    )
                )
            }
        } catch (_: Exception) {}
        return if (out.isEmpty()) defaultShortcuts() else out
    }

    fun saveShortcuts(items: List<ShortcutItem>) {
        val arr = JSONArray()
        for (item in items) {
            arr.put(JSONObject().apply {
                put("title", item.title)
                put("url", item.url)
                put("iconKey", item.iconKey)
                put("colorHex", item.colorHex)
            })
        }
        prefs.edit().putString("shortcuts_v2", arr.toString()).apply()
    }

    fun inferIconKey(url: String): String = when {
        url.contains("google.", true) -> "google"
        url.contains("wikipedia.", true) -> "wiki"
        url.contains("ycombinator.", true) -> "hn"
        url.contains("github.", true) -> "github"
        url.contains("youtube.", true) || url.contains("youtu.be", true) -> "youtube"
        url.contains("reddit.", true) -> "reddit"
        url.contains("duckduckgo.", true) -> "ddg"
        url.contains("claude.ai", true) || url.contains("anthropic.", true) -> "claude"
        else -> "globe"
    }

    private fun defaultShortcuts() = mutableListOf(
        ShortcutItem("Google", "https://www.google.com", "google", "#191918"),
        ShortcutItem("Wikipedia", "https://en.m.wikipedia.org", "wiki", "#191918"),
        ShortcutItem("Hacker News", "https://news.ycombinator.com", "hn", "#D97757"),
        ShortcutItem("GitHub", "https://github.com", "github", "#191918"),
        ShortcutItem("YouTube", "https://www.youtube.com", "youtube", "#B83A3A"),
        ShortcutItem("Reddit", "https://www.reddit.com", "reddit", "#D97757"),
        ShortcutItem("DuckDuckGo", "https://html.duckduckgo.com/html/", "ddg", "#D97757"),
        ShortcutItem("Claude Docs", "https://docs.anthropic.com", "claude", "#D97757")
    )
}

data class SerpResultItem(
    val title: String,
    val url: String,
    val displayUrl: String,
    val snippet: String
)

/**
 * Provides Chrome-grade autocomplete search suggestions and reliable SSR Search Results formatting
 * when strict JS-challenge walls (Google enablejs / Brave 429 / Ecosia 403) block direct SERP HTML.
 */
object SmartSearchService {

    private const val UA = "Mozilla/5.0 (Android 14; Mobile; rv:132.0) Gecko/132.0 Firefox/132.0"

    /**
     * Fetches instant OpenSearch type-ahead suggestions as the user types in the Omnibox.
     */
    fun fetchSuggestions(query: String): List<String> {
        val trimmed = query.trim()
        if (trimmed.length < 2) return emptyList()
        return try {
            val encoded = URLEncoder.encode(trimmed, "UTF-8")
            val endpoint = "https://suggestqueries.google.com/complete/search?client=firefox&q=$encoded"
            val conn = (URL(endpoint).openConnection() as HttpURLConnection).apply {
                connectTimeout = 2500
                readTimeout = 2500
                setRequestProperty("User-Agent", UA)
            }
            if (conn.responseCode == 200) {
                val text = BufferedReader(InputStreamReader(conn.inputStream, "UTF-8")).use { it.readText() }
                val root = JSONArray(text)
                val items = root.optJSONArray(1) ?: return emptyList()
                val res = mutableListOf<String>()
                for (i in 0 until items.length().coerceAtMost(6)) {
                    val s = items.optString(i).trim()
                    if (s.isNotEmpty()) res.add(s)
                }
                res
            } else {
                emptyList()
            }
        } catch (_: Exception) {
            emptyList()
        }
    }

    /**
     * Unwraps redirect/proxy search links (e.g. DuckDuckGo //duckduckgo.com/l/?uddg=...) into real URLs.
     */
    fun unwrapSearchUrl(rawUrl: String): String {
        var fixed = rawUrl.trim()
        if (fixed.startsWith("//")) {
            fixed = "https:$fixed"
        }
        if (fixed.contains("duckduckgo.com/l/") && fixed.contains("uddg=")) {
            try {
                val uri = Uri.parse(fixed)
                val target = uri.getQueryParameter("uddg")
                if (!target.isNullOrBlank()) {
                    return target
                }
            } catch (_: Exception) {}
        }
        return fixed
    }

    /**
     * Detects whether a loaded search page hit a JS-challenge or anti-bot wall rather than real results.
     */
    fun isSearchWallOrEmpty(engine: SearchEngine, loadedUrl: String, pageText: String): Boolean {
        if (loadedUrl.contains("/enablejs") ||
            loadedUrl.contains("google.com/sorry") ||
            loadedUrl.contains("httpservice/retry")
        ) {
            return true
        }
        val lower = pageText.lowercase()
        if (lower.contains("please click here if you are not redirected") ||
            lower.contains("enable javascript") ||
            lower.contains("turn on javascript") ||
            lower.contains("unusual traffic") ||
            lower.contains("are you a robot") ||
            lower.contains("just a moment...") ||
            lower.contains("verify you are human")
        ) {
            return true
        }
        return when (engine) {
            SearchEngine.GOOGLE -> pageText.trim().length < 220
            SearchEngine.BRAVE, SearchEngine.ECOSIA -> pageText.trim().length < 220 || lower.contains("429") || lower.contains("403 forbidden")
            else -> false
        }
    }

    /**
     * Fetches real search results via clean SSR DuckDuckGo HTML endpoint (with Bing SSR backup)
     * & formats a Chrome-style mobile SERP HTML page for our pure-Rust rendering engine.
     */
    fun fetchFormattedChromeSerpHtml(query: String, activeEngine: SearchEngine): String {
        val results = if (activeEngine == SearchEngine.BING) {
            fetchBingSsrResults(query).ifEmpty { fetchDuckDuckGoSsrResults(query) }
        } else {
            fetchDuckDuckGoSsrResults(query).ifEmpty { fetchBingSsrResults(query) }
        }
        val encodedQ = escapeHtml(query)
        val engineName = escapeHtml(activeEngine.displayName)

        val cardsHtml = if (results.isEmpty()) {
            """
            <div class="empty-card">
                <h2>No results retrieved for "$encodedQ"</h2>
                <p>Check your internet connection or try tapping Retry / opening DuckDuckGo directly.</p>
                <p><a href="https://html.duckduckgo.com/html/?q=${URLEncoder.encode(query, "UTF-8")}">Open DuckDuckGo Direct &rarr;</a></p>
            </div>
            """.trimIndent()
        } else {
            results.joinToString("\n") { item ->
                val safeTitle = escapeHtml(item.title)
                val safeHref = escapeHtml(item.url)
                val safeUrl = escapeHtml(item.displayUrl)
                val safeSnippet = escapeHtml(item.snippet)
                """
                <div class="result-card">
                    <div class="result-url">$safeUrl</div>
                    <div class="result-title-wrap"><a class="result-title" href="$safeHref">$safeTitle</a></div>
                    <p class="result-snippet">$safeSnippet</p>
                </div>
                """.trimIndent()
            }
        }

        val altBingUrl = "https://www.bing.com/search?q=${URLEncoder.encode(query, "UTF-8")}"
        val altWikiUrl = "https://en.m.wikipedia.org/w/index.php?search=${URLEncoder.encode(query, "UTF-8")}"

        return """
        <!DOCTYPE html>
        <html>
        <head>
            <title>$encodedQ - Nirvana Search</title>
            <style>
                body { background-color: #FAF9F5; color: #191918; font-size: 15px; margin: 0; padding: 0; }
                .serp-header { background-color: #FFFFFF; padding: 12px 14px; border-bottom: 1px solid #E5E0D5; }
                .serp-badge { color: #D97757; font-size: 13px; font-weight: bold; }
                .serp-sub { color: #6B6860; font-size: 12px; margin-top: 2px; }
                .nav-tabs { background-color: #FAF9F5; padding: 8px 14px 10px 14px; border-bottom: 1px solid #E5E0D5; }
                .nav-pill { color: #C26142; font-size: 13px; font-weight: bold; margin-right: 14px; text-decoration: none; }
                .results-wrap { padding: 10px 12px 24px 12px; }
                .result-card { background-color: #FFFFFF; padding: 12px 14px; margin-bottom: 10px; border: 1px solid #E5E0D5; border-radius: 12px; }
                .result-url { color: #2F6F4E; font-size: 12px; margin-bottom: 4px; }
                .result-title-wrap { margin-bottom: 4px; }
                .result-title { color: #191918; font-size: 17px; font-weight: bold; text-decoration: underline; }
                .result-snippet { color: #524F49; font-size: 14px; margin: 4px 0 0 0; }
                .empty-card { background-color: #FFFFFF; padding: 18px; margin: 14px; border-radius: 12px; border: 1px solid #E5E0D5; }
            </style>
        </head>
        <body>
            <div class="serp-header">
                <div class="serp-badge">Nirvana Search &middot; "$encodedQ"</div>
                <div class="serp-sub">${results.size} instant web results via $engineName mode</div>
            </div>
            <div class="nav-tabs">
                <a class="nav-pill" href="$altBingUrl">Try Bing Results</a>
                <a class="nav-pill" href="$altWikiUrl">Search Wikipedia</a>
            </div>
            <div class="results-wrap">
                $cardsHtml
            </div>
        </body>
        </html>
        """.trimIndent()
    }

    private fun fetchDuckDuckGoSsrResults(query: String): List<SerpResultItem> {
        val list = mutableListOf<SerpResultItem>()
        try {
            val encoded = URLEncoder.encode(query, "UTF-8")
            val endpoint = "https://html.duckduckgo.com/html/?q=$encoded"
            val conn = (URL(endpoint).openConnection() as HttpURLConnection).apply {
                connectTimeout = 8000
                readTimeout = 8000
                setRequestProperty("User-Agent", UA)
                setRequestProperty("Accept", "text/html,application/xhtml+xml,*/*")
                setRequestProperty("Accept-Language", "en-US,en;q=0.9")
            }
            if (conn.responseCode in 200..299) {
                val html = BufferedReader(InputStreamReader(conn.inputStream, "UTF-8")).use { it.readText() }
                val blockPattern = Pattern.compile(
                    "<a[^>]*class=\"[^\"]*result__a[^\"]*\"[^>]*href=\"([^\"]+)\"[^>]*>(.*?)</a>(.*?)(?=<a[^>]*class=\"[^\"]*result__a|\\z)",
                    Pattern.DOTALL
                )
                val snippetPattern = Pattern.compile(
                    "class=\"[^\"]*result__snippet[^\"]*\"[^>]*>(.*?)</(?:a|div|span)>",
                    Pattern.DOTALL
                )
                val matcher = blockPattern.matcher(html)
                while (matcher.find() && list.size < 12) {
                    val rawHref = decodeHtmlBasic(matcher.group(1) ?: "")
                    val url = unwrapSearchUrl(rawHref)
                    val title = cleanTagText(matcher.group(2) ?: "")
                    val tail = matcher.group(3) ?: ""
                    val sm = snippetPattern.matcher(tail)
                    val snippet = if (sm.find()) cleanTagText(sm.group(1) ?: "") else ""
                    val displayUrl = try {
                        val u = Uri.parse(url)
                        val host = u.host ?: url
                        val path = u.path?.takeIf { it.length > 1 }?.take(36) ?: ""
                        "$host$path"
                    } catch (_: Exception) {
                        url
                    }
                    if (title.isNotEmpty() && url.startsWith("http")) {
                        list.add(SerpResultItem(title, url, displayUrl, snippet))
                    }
                }
            }
        } catch (_: Exception) {}
        return list
    }

    private fun fetchBingSsrResults(query: String): List<SerpResultItem> {
        val list = mutableListOf<SerpResultItem>()
        try {
            val encoded = URLEncoder.encode(query, "UTF-8")
            val endpoint = "https://www.bing.com/search?q=$encoded"
            val conn = (URL(endpoint).openConnection() as HttpURLConnection).apply {
                connectTimeout = 8000
                readTimeout = 8000
                setRequestProperty("User-Agent", UA)
                setRequestProperty("Accept", "text/html,application/xhtml+xml,*/*")
                setRequestProperty("Accept-Language", "en-US,en;q=0.9")
            }
            if (conn.responseCode in 200..299) {
                val html = BufferedReader(InputStreamReader(conn.inputStream, "UTF-8")).use { it.readText() }
                val algoPattern = Pattern.compile(
                    "<li[^>]*class=\"[^\"]*b_algo[^\"]*\"[^>]*>(.*?)</li>",
                    Pattern.DOTALL
                )
                val linkPattern = Pattern.compile("<h2[^>]*>.*?<a[^>]*href=\"([^\"]+)\"[^>]*>(.*?)</a>.*?</h2>", Pattern.DOTALL)
                val pPattern = Pattern.compile("<p[^>]*>(.*?)</p>", Pattern.DOTALL)
                val algoMatcher = algoPattern.matcher(html)
                while (algoMatcher.find() && list.size < 10) {
                    val block = algoMatcher.group(1) ?: continue
                    val lm = linkPattern.matcher(block)
                    if (!lm.find()) continue
                    val href = decodeHtmlBasic(lm.group(1) ?: "")
                    val title = cleanTagText(lm.group(2) ?: "")
                    val pm = pPattern.matcher(block)
                    val snippet = if (pm.find()) cleanTagText(pm.group(1) ?: "") else ""
                    if (title.isNotEmpty() && href.startsWith("http")) {
                        val disp = try { Uri.parse(href).host ?: href } catch (_: Exception) { href }
                        list.add(SerpResultItem(title, href, disp, snippet))
                    }
                }
            }
        } catch (_: Exception) {}
        return list
    }

    private fun cleanTagText(raw: String): String {
        val noTags = raw.replace(Regex("<[^>]+>"), " ")
        return decodeHtmlBasic(noTags).replace(Regex("\\s+"), " ").trim()
    }

    private fun decodeHtmlBasic(input: String): String {
        return input
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&ndash;", "-")
            .replace("&mdash;", "--")
            .replace("&hellip;", "...")
            .replace("&nbsp;", " ")
    }

    private fun escapeHtml(input: String): String {
        val sb = StringBuilder(input.length)
        for (c in input) {
            when (c) {
                '<' -> sb.append("&lt;")
                '>' -> sb.append("&gt;")
                '&' -> sb.append("&amp;")
                '"' -> sb.append("&quot;")
                else -> sb.append(c)
            }
        }
        return sb.toString()
    }
}
