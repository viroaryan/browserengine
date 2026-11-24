use nirvana_core::geometry::{Point, Size};
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_hit_test_nested_element() {
    let html = r#"
        <html>
            <body>
                <div id="container" style="width: 200px; height: 200px;">
                    <a id="link" href="https://example.com" style="display: block; width: 100px; height: 50px;">Click</a>
                </div>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).expect("root");

    // Hit test inside the <a> tag at (x: 20, y: 20)
    let hit_node = layout_root.hit_test(Point::new(20.0, 20.0));
    assert!(hit_node.is_some());
    let hit_id = hit_node.unwrap();
    // Verify hit node tag is 'a' or text child of 'a'
    assert!(
        dom.tag_name(hit_id) == Some("a")
            || dom.parent(hit_id).and_then(|p| dom.tag_name(p)) == Some("a")
    );

    // Hit test completely outside container at (x: 500, y: 500)
    let outside_hit = layout_root.hit_test(Point::new(500.0, 500.0));
    assert!(outside_hit.is_none() || outside_hit == layout_root.node_id());
}

#[test]
fn test_hit_test_inline_link_on_text() {
    let html = r#"
        <html>
            <body>
                <p><a id="inline-link" href="https://nirvanabrowser.org">Nirvana</a></p>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).expect("root");

    // Hit test directly on the inline text at (x: 10, y: 10)
    let hit_node = layout_root.hit_test(Point::new(10.0, 10.0));
    assert!(hit_node.is_some());
    let hit_id = hit_node.unwrap();
    assert!(
        dom.tag_name(hit_id) == Some("a")
            || dom.parent(hit_id).and_then(|p| dom.tag_name(p)) == Some("a")
    );
}

#[test]
fn test_hit_test_overlapping_children_reverse_order() {
    // Two blocks inside body; child 2 is placed after child 1 in DOM order
    let html = r#"
        <html>
            <body>
                <div id="first" style="width: 100px; height: 100px;"></div>
                <div id="second" style="width: 100px; height: 100px; margin-top: -50px;"></div>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).expect("root");

    // Point at (x: 50, y: 60) is inside both first (y: 0..100) and second (y: 50..150).
    // Because second is painted after first (on top), hit_test must return the second div!
    let hit_node = layout_root.hit_test(Point::new(50.0, 60.0));
    assert!(hit_node.is_some());
    let hit_id = hit_node.unwrap();
    assert_eq!(dom.get_attribute(hit_id, "id"), Some("second"));
}
