use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_hit_test_detects_anchor_link_and_href() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<div style="padding: 20px;"><a href="https://nirvanabrowser.org" style="width: 100px; height: 30px;">Open Nirvana</a></div>"#;
    engine.load_html(html, None);
    engine.render();

    // Tap inside the link at (x: 25, y: 25)
    let result = engine.hit_test(25.0, 25.0);
    assert!(result.is_some());
    let hit = result.unwrap();
    assert_eq!(hit.tag, "a");
    assert_eq!(hit.href.as_deref(), Some("https://nirvanabrowser.org"));
}

#[test]
fn test_engine_hit_test_with_scroll_offset() {
    let mut engine = NirvanaEngine::new(400, 300);
    // Spacer of 200px, followed by link, followed by 500px spacer so document is scrollable
    let html = r#"<div><div style="height: 200px;"></div><a href="https://example.com">Scrolled Link</a><div style="height: 500px;"></div></div>"#;
    engine.load_html(html, None);
    engine.render();

    // Before scroll, link is around y = 200..218. Tapping at screen y = 10 hits the spacer div.
    let before_hit = engine.hit_test(10.0, 10.0);
    assert!(before_hit.is_some());
    assert_ne!(before_hit.unwrap().tag, "a");

    // Scroll down 200px: link now shifts to screen y = 0..18
    engine.scroll_by(0.0, 200.0);
    let after_hit = engine.hit_test(10.0, 10.0);
    assert!(after_hit.is_some());
    let hit = after_hit.unwrap();
    assert_eq!(hit.tag, "a");
    assert_eq!(hit.href.as_deref(), Some("https://example.com"));
}
