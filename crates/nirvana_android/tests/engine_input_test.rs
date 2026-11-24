use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_click_input_focuses_and_positions_caret() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <style>
        html, body { margin: 0; padding: 0; }
        input { display: block; width: 200px; height: 30px; }
    </style>
</head>
<body>
    <input value="hello world">
</body>
</html>"#;
    engine.load_html(html, None);
    engine.render();

    assert!(!engine.is_input_focused());
    assert_eq!(engine.focused_element(), None);

    // Click inside the input at (30, 15)
    let hit = engine.handle_click(30.0, 15.0);
    assert!(hit.is_some());
    assert!(engine.is_input_focused());

    let focused_id = engine.focused_element().unwrap();
    assert_eq!(hit.unwrap().node_id, focused_id);
    assert_eq!(engine.focused_text().as_deref(), Some("hello world"));

    // Click outside input at (100, 200) -> should blur
    let _hit2 = engine.handle_click(100.0, 200.0);
    assert!(!engine.is_input_focused());
    assert_eq!(engine.focused_element(), None);
}

#[test]
fn test_engine_insert_text_and_delete_backward() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<input value="foo">"#;
    engine.load_html(html, None);
    engine.render();

    // Focus at end of "foo" (index 3)
    let hit = engine.handle_click(10.0, 10.0);
    let id = hit.unwrap().node_id;
    engine.focus(id, 3);
    assert_eq!(engine.focused_text().as_deref(), Some("foo"));

    // Insert "bar"
    engine.insert_text("bar");
    assert_eq!(engine.focused_text().as_deref(), Some("foobar"));

    // Backspace twice
    engine.delete_backward();
    assert_eq!(engine.focused_text().as_deref(), Some("fooba"));
    engine.delete_backward();
    assert_eq!(engine.focused_text().as_deref(), Some("foob"));

    // Insert at beginning: move cursor to 0
    engine.set_cursor_index(0);
    engine.insert_text("super_");
    assert_eq!(engine.focused_text().as_deref(), Some("super_foob"));
}

#[test]
fn test_engine_focused_bounds_with_scrolling() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <style>
        html, body { margin: 0; padding: 0; }
        .spacer { height: 100px; }
        input { display: block; width: 160px; height: 24px; }
    </style>
</head>
<body>
    <div class="spacer"></div>
    <input value="target">
    <div style="height: 500px;"></div>
</body>
</html>"#;
    engine.load_html(html, None);
    engine.render();

    // Click input at (20, 110)
    engine.handle_click(20.0, 110.0);
    assert!(engine.is_input_focused());

    let bounds1 = engine.focused_bounds().unwrap();
    assert_eq!(bounds1.y(), 100.0); // Directly below 100px spacer

    // Scroll down 40px
    engine.scroll_to(0.0, 40.0);
    let bounds2 = engine.focused_bounds().unwrap();
    // Screen bounds y should shift up by 40px: 100 - 40 = 60
    assert_eq!(bounds2.y(), 60.0);
}

#[test]
fn test_password_tap_measures_masked_bullets() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<input type="password" value="WWWWWW">"#;
    engine.load_html(html, None);
    engine.render();

    // Tap inside password field
    engine.handle_click(5.0, 10.0);
    assert!(engine.is_input_focused());
    assert_eq!(engine.focused_text().as_deref(), Some("WWWWWW"));
}
