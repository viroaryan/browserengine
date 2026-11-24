use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_executes_inline_script_and_updates_dom() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <body>
                <h1 id="title">Initial Title</h1>
                <script>
                    document.getElementById('title').textContent = 'Executed from Script!';
                </script>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    let dom = engine.dom().expect("dom exists");
    let mut title_text = String::new();
    for node in dom.nodes() {
        if dom.get_attribute(node, "id") == Some("title") {
            for child in dom.children(node) {
                if let Some(n) = dom.get_node(child)
                    && let nirvana_dom::NodeData::Text(ref t) = n.data
                {
                    title_text.push_str(t);
                }
            }
        }
    }
    assert_eq!(title_text, "Executed from Script!");
}

#[test]
fn test_engine_script_modifies_style_and_layout() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <body>
                <div id="box" style="width: 100px; height: 50px;">Box</div>
                <script>
                    document.getElementById('box').style.width = '250px';
                </script>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    let layout = engine.layout_root().expect("layout exists");
    let dom = engine.dom().expect("dom exists");
    let box_id = dom
        .nodes()
        .find(|&n| dom.get_attribute(n, "id") == Some("box"))
        .expect("box element exists");

    fn find_box(
        b: &nirvana_layout::LayoutBox,
        id: nirvana_dom::NodeId,
    ) -> Option<&nirvana_layout::LayoutBox> {
        if b.node_id() == Some(id) {
            return Some(b);
        }
        for child in &b.children {
            if let Some(found) = find_box(child, id) {
                return Some(found);
            }
        }
        None
    }

    let box_layout = find_box(layout, box_id).expect("box layout found");
    assert_eq!(box_layout.dimensions.content.width(), 250.0);
}

#[test]
fn test_engine_handle_click_dispatches_js_event_and_re_renders() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <body style="margin: 0; padding: 0;">
                <button id="counter-btn" style="width: 100px; height: 40px;">0</button>
                <script>
                    var count = 0;
                    var btn = document.getElementById('counter-btn');
                    btn.addEventListener('click', function() {
                        count++;
                        btn.textContent = String(count);
                    });
                </script>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    // Tap button at (20, 20)
    let hit = engine.handle_click(20.0, 20.0);
    assert!(hit.is_some());

    // Check button text in DOM updated to "1"
    let dom = engine.dom().expect("dom exists");
    let btn_id = dom
        .nodes()
        .find(|&n| dom.get_attribute(n, "id") == Some("counter-btn"))
        .expect("button exists");

    let mut btn_text = String::new();
    for child in dom.children(btn_id) {
        if let Some(n) = dom.get_node(child)
            && let nirvana_dom::NodeData::Text(ref t) = n.data
        {
            btn_text.push_str(t);
        }
    }
    assert_eq!(btn_text, "1");

    // Tap button a second time
    let _ = engine.handle_click(20.0, 20.0);
    let dom2 = engine.dom().expect("dom exists");
    let mut btn_text2 = String::new();
    for child in dom2.children(btn_id) {
        if let Some(n) = dom2.get_node(child)
            && let nirvana_dom::NodeData::Text(ref t) = n.data
        {
            btn_text2.push_str(t);
        }
    }
    assert_eq!(btn_text2, "2");
}

#[test]
fn test_engine_evaluate_js_api() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"<html><body><h1 id="header">Old</h1></body></html>"#;
    engine.load_html(html, None);
    engine.render();

    let res = engine.evaluate_js("document.getElementById('header').textContent = 'Brand New';");
    assert!(res.is_ok());

    let dom = engine.dom().expect("dom exists");
    let mut header_text = String::new();
    for node in dom.nodes() {
        if dom.get_attribute(node, "id") == Some("header") {
            for child in dom.children(node) {
                if let Some(n) = dom.get_node(child)
                    && let nirvana_dom::NodeData::Text(ref t) = n.data
                {
                    header_text.push_str(t);
                }
            }
        }
    }
    assert_eq!(header_text, "Brand New");
}

#[test]
fn test_engine_step_timers_api() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <body>
                <div id="status">pending</div>
                <script>
                    setTimeout(function() {
                        document.getElementById('status').textContent = 'completed';
                    }, 100);
                </script>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    // Step 50ms - should not fire yet
    let count = engine.step_timers(50);
    assert_eq!(count, 0);

    let dom1 = engine.dom().expect("dom exists");
    let mut status_text1 = String::new();
    for node in dom1.nodes() {
        if dom1.get_attribute(node, "id") == Some("status") {
            for child in dom1.children(node) {
                if let Some(n) = dom1.get_node(child)
                    && let nirvana_dom::NodeData::Text(ref t) = n.data
                {
                    status_text1.push_str(t);
                }
            }
        }
    }
    assert_eq!(status_text1, "pending");

    // Step another 60ms (total 110ms) - timer fires
    let count2 = engine.step_timers(60);
    assert_eq!(count2, 1);

    let dom2 = engine.dom().expect("dom exists");
    let mut status_text2 = String::new();
    for node in dom2.nodes() {
        if dom2.get_attribute(node, "id") == Some("status") {
            for child in dom2.children(node) {
                if let Some(n) = dom2.get_node(child)
                    && let nirvana_dom::NodeData::Text(ref t) = n.data
                {
                    status_text2.push_str(t);
                }
            }
        }
    }
    assert_eq!(status_text2, "completed");
}

#[test]
fn test_form_input_and_js_value_synchronization() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <body style="margin: 0px;">
                <input id="user-input" value="hello" style="width: 200px; height: 40px;">
                <div id="output">idle</div>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    // 1. User taps input and positions caret at end, then types "_world"
    let _ = engine.handle_click(10.0, 10.0);
    engine.set_cursor_index(5);
    engine.insert_text("_world");

    // 2. JavaScript reads the input value via document.getElementById('user-input').value
    let res = engine
        .evaluate_js("document.getElementById('user-input').value")
        .expect("eval input value");
    assert_eq!(res, "hello_world");

    // 3. JavaScript modifies another element, triggering engine re-render
    let _ = engine.evaluate_js("document.getElementById('output').textContent = 'sync_done';");

    // 4. Verify that form control still contains "hello_world" and was not obliterated
    assert_eq!(engine.focused_text().as_deref(), Some("hello_world"));
    let res2 = engine
        .evaluate_js("document.getElementById('user-input').value")
        .expect("eval input value after re-render");
    assert_eq!(res2, "hello_world");
}

#[test]
fn test_dynamic_style_tag_injection_updates_cascade() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <head>
                <style>
                    #box { width: 100px; height: 100px; background-color: #ff0000; }
                </style>
            </head>
            <body style="margin: 0px;">
                <div id="box"></div>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    // Initial width is 100px
    let dom = engine.dom().expect("dom exists");
    let box_id = dom
        .nodes()
        .find(|&n| dom.get_attribute(n, "id") == Some("box"))
        .expect("box element exists");

    fn find_box(
        b: &nirvana_layout::LayoutBox,
        id: nirvana_dom::NodeId,
    ) -> Option<&nirvana_layout::LayoutBox> {
        if b.node_id() == Some(id) {
            return Some(b);
        }
        for child in &b.children {
            if let Some(found) = find_box(child, id) {
                return Some(found);
            }
        }
        None
    }

    let initial_box = find_box(engine.layout_root().unwrap(), box_id).unwrap();
    assert_eq!(initial_box.dimensions.content.width(), 100.0);

    // Dynamically inject a <style> tag setting #box { width: 350px; }
    let _ = engine.evaluate_js(
        r#"
        var style = document.createElement('style');
        style.textContent = '#box { width: 350px; }';
        document.body.appendChild(style);
    "#,
    );

    // Re-computed layout must now have width = 350px
    let updated_box = find_box(engine.layout_root().unwrap(), box_id).unwrap();
    assert_eq!(updated_box.dimensions.content.width(), 350.0);
}

#[test]
fn test_prevent_default_cancels_link_navigation() {
    let mut engine = NirvanaEngine::new(400, 300);
    let html = r#"
        <html>
            <body style="margin: 0px;">
                <a id="nav-link" href="https://nirvanabrowser.org" style="display: block; width: 150px; height: 50px;">Click Me</a>
                <script>
                    var link = document.getElementById('nav-link');
                    link.addEventListener('click', function(e) {
                        e.preventDefault();
                    });
                </script>
            </body>
        </html>
    "#;
    engine.load_html(html, None);
    engine.render();

    // Click on link at (20, 20)
    let hit = engine.handle_click(20.0, 20.0);
    assert!(hit.is_some());
    let hit_res = hit.unwrap();
    // Default was prevented by JS, so href must be None (suppressed navigation)
    assert_eq!(hit_res.href, None);
    assert_eq!(hit_res.tag, "a");
}
