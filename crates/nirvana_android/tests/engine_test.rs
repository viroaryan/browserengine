use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_lifecycle_and_rendering() {
    let mut engine = NirvanaEngine::new(400, 300);
    assert_eq!(engine.dimensions(), (400, 300));
    assert_eq!(engine.pixels().len(), 400 * 300 * 4);

    let html = r#"<div style="background-color: #ff0000; width: 100px; height: 100px;"></div>"#;
    engine.load_html(html, None);
    engine.render();

    // Verify non-empty pixel data was rendered
    let pixels = engine.pixels();
    assert_eq!(pixels.len(), 400 * 300 * 4);
    // At coordinate (10, 10), red color should be present: R=255, G=0, B=0, A=255
    let idx = (10 * 400 + 10) * 4;
    assert_eq!(pixels[idx], 255);
    assert_eq!(pixels[idx + 1], 0);
    assert_eq!(pixels[idx + 2], 0);
    assert_eq!(pixels[idx + 3], 255);
}

#[test]
fn test_engine_resize() {
    let mut engine = NirvanaEngine::new(200, 200);
    assert_eq!(engine.dimensions(), (200, 200));

    engine.resize(500, 400);
    assert_eq!(engine.dimensions(), (500, 400));
    assert_eq!(engine.pixels().len(), 500 * 400 * 4);
}
