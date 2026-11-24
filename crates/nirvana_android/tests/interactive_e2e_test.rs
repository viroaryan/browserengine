use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_end_to_end_interactive_scrolling_and_hit_testing() {
    let mut engine = NirvanaEngine::new(400, 300);

    let html = r#"
        <html>
            <body style="margin: 0px;">
                <div id="header" style="height: 100px; width: 400px; background-color: #ff0000;">
                    <a id="nav-home" href="https://nirvanabrowser.org/home">Home</a>
                </div>
                <div id="spacer" style="height: 300px; width: 400px; background-color: #00ff00;"></div>
                <div id="footer" style="height: 200px; width: 400px; background-color: #0000ff;">
                    <a id="nav-docs" href="https://nirvanabrowser.org/docs">Documentation</a>
                </div>
            </body>
        </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    // 1. Initial viewport state
    assert_eq!(engine.scroll_offset(), (0.0, 0.0));
    let (max_x, max_y) = engine.max_scroll();
    assert_eq!(max_x, 0.0);
    // Total document height = 100 (header) + 300 (spacer) + 200 (footer) = 600px
    // Viewport height = 300px => max_scroll_y = 300px
    assert_eq!(max_y, 300.0);

    // Initial pixel check: header is red (#ff0000) at screen (x: 200, y: 50)
    let p_initial = pixel_at(&engine, 200, 50);
    assert_eq!(p_initial, Rgba::new(255, 0, 0, 255));

    // Hit test header link at (x: 10, y: 10)
    let hit_header = engine.hit_test(10.0, 10.0);
    assert!(hit_header.is_some());
    let hit_h = hit_header.unwrap();
    assert_eq!(hit_h.tag, "a");
    assert_eq!(
        hit_h.href.as_deref(),
        Some("https://nirvanabrowser.org/home")
    );

    // 2. Scroll down 250px (into spacer and top of footer)
    engine.scroll_by(0.0, 250.0);
    assert_eq!(engine.scroll_offset(), (0.0, 250.0));

    // After 250px scroll:
    // Header (y: 0..100) is now at screen y: -250..-150 (offscreen).
    // Spacer (y: 100..400) is now at screen y: -150..150.
    // Screen pixel at (x: 200, y: 50) was doc y: 300 (inside green spacer)!
    let p_scrolled = pixel_at(&engine, 200, 50);
    assert_eq!(p_scrolled, Rgba::new(0, 255, 0, 255));

    // 3. Scroll all the way to max_scroll (300px)
    engine.scroll_to(0.0, 300.0);
    assert_eq!(engine.scroll_offset(), (0.0, 300.0));

    // Footer starts at doc y: 400.
    // With scroll_y = 300, footer is at screen y: 400 - 300 = 100..300.
    // Footer link is inside footer at doc y = 400. At screen y = 110, we tap inside the footer link!
    let hit_footer = engine.hit_test(10.0, 110.0);
    assert!(hit_footer.is_some());
    let hit_f = hit_footer.unwrap();
    assert_eq!(hit_f.tag, "a");
    assert_eq!(
        hit_f.href.as_deref(),
        Some("https://nirvanabrowser.org/docs")
    );

    // 4. Over-scroll clamping checks
    // Scroll past bottom
    engine.scroll_by(0.0, 5000.0);
    assert_eq!(engine.scroll_offset(), (0.0, 300.0));

    // Scroll past top
    engine.scroll_by(0.0, -9999.0);
    assert_eq!(engine.scroll_offset(), (0.0, 0.0));

    // 5. Viewport resize maintains safe scroll bounds
    engine.resize(400, 500);
    let (_new_max_x, new_max_y) = engine.max_scroll();
    // 600 - 500 = 100px max scroll
    assert_eq!(new_max_y, 100.0);
    engine.scroll_to(0.0, 500.0);
    assert_eq!(engine.scroll_offset(), (0.0, 100.0));
}

fn pixel_at(engine: &NirvanaEngine, x: u32, y: u32) -> Rgba {
    let (w, _h) = engine.dimensions();
    let idx = ((y * w + x) * 4) as usize;
    let pixels = engine.pixels();
    Rgba::new(
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    )
}
