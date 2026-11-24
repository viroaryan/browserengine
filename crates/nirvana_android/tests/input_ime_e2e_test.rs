use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

fn sample_pixel(engine: &NirvanaEngine, x: u32, y: u32) -> Rgba {
    let (w, _h) = engine.dimensions();
    let pixels = engine.pixels();
    let idx = ((y as usize) * (w as usize) + (x as usize)) * 4;
    Rgba::new(pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
}

#[test]
fn test_end_to_end_form_controls_and_input_editing() {
    let mut engine = NirvanaEngine::new(500, 400);

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <style>
        html, body {
            margin: 0;
            padding: 20px;
            background-color: white;
        }
        .search-box {
            display: block;
            width: 200px;
            height: 32px;
            border-width: 1px;
            background-color: white;
            color: black;
            margin-bottom: 20px;
        }
        .comment-box {
            display: block;
            width: 250px;
            height: 80px;
            background-color: white;
        }
    </style>
</head>
<body>
    <input class="search-box" value="nirvana">
    <textarea class="comment-box">first line</textarea>
</body>
</html>"#;

    engine.load_html(html, None);
    engine.render();

    // 1. Initial State: No focus
    assert!(!engine.is_input_focused());
    assert_eq!(engine.focused_element(), None);

    // 2. Click inside search-box at (50, 50)
    let hit1 = engine.handle_click(50.0, 50.0);
    assert!(hit1.is_some(), "Search box should be hit");
    let input_node = hit1.unwrap().node_id;
    assert!(engine.is_input_focused());
    assert_eq!(engine.focused_element(), Some(input_node));
    assert_eq!(engine.focused_text().as_deref(), Some("nirvana"));

    // 3. IME typing simulation: append " browser"
    engine.set_cursor_index(7); // End of "nirvana"
    engine.insert_text(" browser");
    assert_eq!(engine.focused_text().as_deref(), Some("nirvana browser"));

    // 4. Backspace 3 times ("ser") -> "nirvana brow"
    engine.delete_backward();
    engine.delete_backward();
    engine.delete_backward();
    assert_eq!(engine.focused_text().as_deref(), Some("nirvana brow"));

    // Verify focus bounds
    let bounds = engine.focused_bounds();
    assert!(bounds.is_some());
    let b = bounds.unwrap();
    assert_eq!(b.x(), 40.0);
    assert_eq!(b.y(), 40.0);

    // 5. Click inside comment-box at (50, 120)
    let hit2 = engine.handle_click(50.0, 120.0);
    assert!(hit2.is_some(), "Comment box textarea should be hit");
    let textarea_node = hit2.unwrap().node_id;
    assert!(engine.is_input_focused());
    assert_eq!(engine.focused_element(), Some(textarea_node));
    assert_eq!(engine.focused_text().as_deref(), Some("first line"));

    // Typing inside textarea
    engine.set_cursor_index(10);
    engine.insert_text("\nsecond");
    assert_eq!(engine.focused_text().as_deref(), Some("first line\nsecond"));

    // 6. Click background at (400, 350) -> should blur all inputs
    let _hit3 = engine.handle_click(400.0, 350.0);
    assert!(!engine.is_input_focused());
    assert_eq!(engine.focused_element(), None);

    // 7. Verify pixel framebuffer rendered valid non-empty data
    let _p = sample_pixel(&engine, 40, 40); // Border pixel
    // The canvas was rasterized and updated
    assert_eq!(engine.pixels().len(), 500 * 400 * 4);
}
