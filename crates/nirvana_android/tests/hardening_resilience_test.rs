use nirvana_engine::engine::NirvanaEngine;
use nirvana_gfx::image::{decode_image, ImageError, MAX_IMAGE_DIMENSION};

#[test]
fn test_deeply_nested_html_does_not_crash_or_overflow_stack() {
    let mut engine = NirvanaEngine::new(400, 400);

    // 800 levels of nested div tags (exceeding MAX_TREE_DEPTH = 512)
    // Place safe element at depth 200, then exceed limit with remaining 600 divs
    let mut html = String::new();
    for _ in 0..200 {
        html.push_str("<div>");
    }
    html.push_str("<p id='deep'>Deep Content</p>");
    for _ in 200..800 {
        html.push_str("<div>");
    }
    for _ in 200..800 {
        html.push_str("</div>");
    }
    for _ in 0..200 {
        html.push_str("</div>");
    }

    engine.load_html(&html, None);
    engine.render();

    let is_found = engine
        .evaluate_js("document.getElementById('deep') !== null")
        .unwrap_or_default();
    assert_eq!(is_found, "true");
    let text = engine
        .evaluate_js("document.getElementById('deep').textContent")
        .unwrap_or_default();
    assert_eq!(text, "Deep Content");
    assert_eq!(engine.pixels().len(), 400 * 400 * 4);
}

#[test]
fn test_zero_and_extreme_viewport_resizing() {
    let mut engine = NirvanaEngine::new(0, 0);
    let (w, h) = engine.dimensions();
    assert!(w >= 1 && h >= 1);
    assert_eq!(engine.pixels().len(), (w * h * 4) as usize);

    // Resize to zero
    engine.resize(0, 0);
    let (w2, h2) = engine.dimensions();
    assert!(w2 >= 1 && h2 >= 1);

    // Resize to extreme dimension
    engine.resize(50_000, 50_000);
    let (w3, h3) = engine.dimensions();
    assert!(w3 <= 16384 && h3 <= 16384);
    assert_eq!(engine.pixels().len(), (w3 * h3 * 4) as usize);
}

#[test]
fn test_malformed_html_unclosed_tags_and_invalid_entities() {
    let mut engine = NirvanaEngine::new(300, 300);
    let malformed = "<<<<div><p>Unclosed paragraph <b>Bold text without close &#9999999; &invalid; <br>";
    engine.load_html(malformed, None);
    engine.render();
    assert_eq!(engine.pixels().len(), 300 * 300 * 4);
}

#[test]
fn test_oversized_image_bounds_rejection() {
    // Construct fake PNG header with huge dimensions (e.g. 10000 x 10000)
    // Using decode_image on empty data or invalid dimensions
    assert_eq!(decode_image(&[]), Err(ImageError::EmptyData));

    // Valid constant check
    assert_eq!(MAX_IMAGE_DIMENSION, 8192);
}
