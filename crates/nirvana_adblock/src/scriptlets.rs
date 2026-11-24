//! Scriptlet defusers injected into pages to prevent anti-adblock traps and script errors.

/// Generates a sovereign JavaScript defuser script injected before page scripts execute.
pub fn generate_defuser_script() -> &'static str {
    r#"
    (function() {
        // No-op Google Analytics stubs
        window.ga = window.ga || function() {};
        window.gtag = window.gtag || function() {};
        window._gaq = window._gaq || [];
        if (!window._gaq.push) { window._gaq.push = function() {}; }
        window.google_tag_manager = window.google_tag_manager || {};

        // No-op Facebook Pixel stubs
        window.fbq = window.fbq || function() {};

        // Anti-adblock detection neutralizers
        window.__adblocker = false;
        window.canRunAds = true;
        window.isAdBlockActive = false;
    })();
    "#
}
