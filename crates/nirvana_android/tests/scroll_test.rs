use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_scroll_clamping_and_bounds() {
    let mut engine = NirvanaEngine::new(400, 300);
    // Tall document of 1000px height
    let html = r#"<div style="height: 1000px; width: 400px; background-color: #ff0000;"></div>"#;
    engine.load_html(html, None);
    engine.render();

    assert_eq!(engine.scroll_offset(), (0.0, 0.0));
    let (max_x, max_y) = engine.max_scroll();
    assert_eq!(max_x, 0.0);
    assert_eq!(max_y, 700.0); // 1000 - 300

    // Scroll down 200px
    engine.scroll_by(0.0, 200.0);
    assert_eq!(engine.scroll_offset(), (0.0, 200.0));

    // Scroll past bottom: must clamp to max_y
    engine.scroll_by(0.0, 1000.0);
    assert_eq!(engine.scroll_offset(), (0.0, 700.0));

    // Scroll past top: must clamp to 0.0
    engine.scroll_by(0.0, -999.0);
    assert_eq!(engine.scroll_offset(), (0.0, 0.0));
}
