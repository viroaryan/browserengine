use nirvana_engine::engine::NirvanaEngine;
use nirvana_style::Display;

#[test]
fn test_adblock_scriptlet_defusers_injected_at_document_start() {
    let mut engine = NirvanaEngine::new(400, 400);

    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <script>
                // Site JS calling Google Analytics without crashing
                window.ga('send', 'pageview');
                window.test_ga_defined = (typeof window.ga === 'function');
                window.test_gtm_defined = (typeof window.google_tag_manager === 'object');
                window.test_adblocker_flag = window.__adblocker;
            </script>
        </head>
        <body>
            <h1>Page with Analytics</h1>
        </body>
        </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    assert_eq!(
        engine.evaluate_js("window.test_ga_defined").unwrap_or_default(),
        "true"
    );
    assert_eq!(
        engine.evaluate_js("window.test_gtm_defined").unwrap_or_default(),
        "true"
    );
    assert_eq!(
        engine.evaluate_js("window.test_adblocker_flag").unwrap_or_default(),
        "false"
    );
}

#[test]
fn test_adblock_cosmetic_filtering_hides_ad_containers() {
    let mut engine = NirvanaEngine::new(600, 600);

    let html = r#"
        <!DOCTYPE html>
        <html>
        <body>
            <div id="content"><p>Main Article Content</p></div>
            <div class="adsbygoogle" style="width: 300px; height: 250px;">
                <span>Sponsored Ad Banner</span>
            </div>
            <div class="advertisement" style="width: 300px; height: 100px;">
                <span>Another Ad</span>
            </div>
        </body>
        </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    // Verify .adsbygoogle has Display::None and is excluded from layout
    let ad_node = engine.find_node_by_class("adsbygoogle").expect("Found .adsbygoogle in DOM");
    let style = engine.style_tree().unwrap().get_style(ad_node).unwrap();
    assert_eq!(style.display, Display::None);
    assert!(!engine.is_node_in_layout(ad_node), ".adsbygoogle should not be in layout tree");

    let adv_node = engine.find_node_by_class("advertisement").expect("Found .advertisement in DOM");
    let adv_style = engine.style_tree().unwrap().get_style(adv_node).unwrap();
    assert_eq!(adv_style.display, Display::None);
    assert!(!engine.is_node_in_layout(adv_node), ".advertisement should not be in layout tree");
}

#[test]
fn test_adblock_toggle_and_custom_rule_management() {
    let mut engine = NirvanaEngine::new(400, 400);

    assert!(engine.is_adblock_enabled());
    assert_eq!(engine.blocked_requests_count(), 0);

    // Add custom filter rule
    engine.add_adblock_rule("||custom-tracker.net^");
    engine.add_adblock_rule("##.custom-sponsored-box");

    let html = r#"
        <div class="custom-sponsored-box">Custom Ad</div>
        <p>Clean text</p>
    "#;
    engine.load_html(html, None);
    engine.render();

    let custom_node = engine.find_node_by_class("custom-sponsored-box").unwrap();
    let style = engine.style_tree().unwrap().get_style(custom_node).unwrap();
    assert_eq!(style.display, Display::None);
    assert!(!engine.is_node_in_layout(custom_node));

    // Toggle adblock off
    engine.set_adblock_enabled(false);
    assert!(!engine.is_adblock_enabled());

    // Re-render without adblock
    engine.load_html(html, None);
    engine.render();

    let custom_node_after = engine.find_node_by_class("custom-sponsored-box").unwrap();
    let style_after = engine.style_tree().unwrap().get_style(custom_node_after).unwrap();
    assert_ne!(style_after.display, Display::None);
    assert!(engine.is_node_in_layout(custom_node_after));
}
