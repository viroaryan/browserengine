package com.nirvana.browser.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BrowserModelsTest {

    @Test
    fun fiveDistinctSearchEnginesConfiguredCleanly() {
        val engines = SearchEngine.values()
        assertEquals(5, engines.size)
        assertEquals(
            listOf("Google", "DuckDuckGo", "Bing", "Brave", "Ecosia"),
            engines.map { it.displayName }
        )
        assertEquals(
            listOf("G", "D", "B", "Br", "E"),
            engines.map { it.shortCode }
        )
        for (engine in engines) {
            assertTrue(engine.searchUrl.startsWith("https://"))
        }
    }

    @Test
    fun searchWallDetectorIdentifiesJsAndCaptchaWalls() {
        assertTrue(
            SmartSearchService.isSearchWallOrEmpty(
                SearchEngine.GOOGLE,
                "https://www.google.com/httpservice/retry/enablejs?sei=abc",
                ""
            )
        )
        assertTrue(
            SmartSearchService.isSearchWallOrEmpty(
                SearchEngine.BRAVE,
                "https://search.brave.com/search?q=test",
                "Please verify you are a human to continue browsing."
            )
        )
        assertTrue(
            SmartSearchService.isSearchWallOrEmpty(
                SearchEngine.GOOGLE,
                "https://www.google.com/search?q=rust",
                "Google requires you to enable JavaScript to continue viewing search results cleanly on this device."
            )
        )
        assertFalse(
            SmartSearchService.isSearchWallOrEmpty(
                SearchEngine.DUCKDUCKGO,
                "https://html.duckduckgo.com/html/?q=rust",
                "Rust Programming Language https://www.rust-lang.org A language empowering everyone to build reliable and efficient software."
            )
        )
    }

    @Test
    fun historyStepPreservesSerpAndCacheState() {
        val step = HistoryStep(
            url = "https://www.google.com/search?q=claude",
            isSearchSerp = true,
            searchQuery = "claude",
            cachedHtml = "<html><body>SERP</body></html>"
        )
        val tab = BrowserTab(id = 1L, isOnHome = false, url = "https://anthropic.com")
        tab.backStack.add(step)
        assertEquals(1, tab.backStack.size)
        assertTrue(tab.backStack.first().isSearchSerp)
        assertEquals("claude", tab.backStack.first().searchQuery)
    }

    @Test
    fun omniboxRouterDistinguishesUrlsLocalhostAndSearchQueries() {
        assertTrue(OmniboxRouter.looksLikeUrl("wikipedia.org"))
        assertTrue(OmniboxRouter.looksLikeUrl("https://docs.anthropic.com"))
        assertTrue(OmniboxRouter.looksLikeUrl("localhost:8080/test"))
        assertEquals("https://wikipedia.org", OmniboxRouter.normalizeUrl("wikipedia.org"))
        assertEquals("http://localhost:8080/test", OmniboxRouter.normalizeUrl("localhost:8080/test"))

        assertFalse(OmniboxRouter.looksLikeUrl("rust programming"))
        assertFalse(OmniboxRouter.looksLikeUrl("claude"))
        assertFalse(OmniboxRouter.looksLikeUrl(".hidden"))
        assertFalse(OmniboxRouter.looksLikeUrl(""))
    }
}
