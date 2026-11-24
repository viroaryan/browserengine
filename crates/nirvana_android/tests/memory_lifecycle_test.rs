use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_rapid_engine_allocation_and_destruction_loop() {
    // Stress test: instantiate, load, render, and drop 50 engine instances in sequence
    for i in 0..50 {
        let mut engine = NirvanaEngine::new(320, 480);
        let html = format!(
            "<!DOCTYPE html><html><body><h1>Page {}</h1><p>Stress test iteration</p></body></html>",
            i
        );
        engine.load_html(&html, Some("h1 { color: #ff0000; } p { font-size: 14px; }"));
        engine.render();

        assert_eq!(engine.pixels().len(), 320 * 480 * 4);
        assert!(engine.frame_metrics().total_frame_time_ms >= 0.0);
        // Engine cleanly dropped here
    }
}

#[test]
fn test_repeated_document_reloads_and_cache_cleanup() {
    let mut engine = NirvanaEngine::new(400, 400);

    // Repeatedly load different documents into the same engine instance
    for i in 0..20 {
        let html = format!(
            "<div><p id='target'>Iteration {}</p><span>Extra content</span></div>",
            i
        );
        engine.load_html(&html, None);
        engine.render();

        let val = engine
            .evaluate_js("document.getElementById('target').textContent")
            .unwrap_or_default();
        assert_eq!(val, format!("Iteration {}", i));
    }

    // Purge caches and shrink allocations
    engine.clear_image_cache();
    engine.shrink_to_fit();

    assert_eq!(engine.pixels().len(), 400 * 400 * 4);
}

#[test]
fn test_storage_and_cookie_isolation_across_clears() {
    let mut engine = NirvanaEngine::new(300, 300);

    engine.load_html(
        "<html><head><script>localStorage.setItem('session', 'active123');</script></head><body></body></html>",
        None,
    );
    engine.render();

    let val = engine.evaluate_js("localStorage.getItem('session')").unwrap_or_default();
    assert_eq!(val, "active123");

    // Clear storage and cookies
    engine.clear_storage();
    engine.clear_cookies();

    let cleared_val = engine.evaluate_js("localStorage.getItem('session')").unwrap_or_default();
    assert_eq!(cleared_val, "null");
}
