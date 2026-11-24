use nirvana_core::geometry::Rect;
use nirvana_html::parse_html;
use nirvana_layout::dimensions::Dimensions;
use nirvana_layout::tree_builder::build_layout_tree;
use nirvana_style::compute::compute_styles;

#[test]
fn test_block_width_auto_expansion() {
    let html = "<div></div>";
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let viewport_containing_block = Dimensions {
        content: Rect::new(0.0, 0.0, 800.0, 600.0),
        ..Default::default()
    };

    layout_root.layout(&viewport_containing_block, &style_tree);

    // html content width expands to fill containing block (800.0)
    assert_eq!(layout_root.dimensions.content.width(), 800.0);
}

#[test]
fn test_block_margin_auto_centering() {
    let html = r#"<div style="width: 400px; margin: 0 auto;"></div>"#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let viewport_containing_block = Dimensions {
        content: Rect::new(0.0, 0.0, 800.0, 600.0),
        ..Default::default()
    };

    layout_root.layout(&viewport_containing_block, &style_tree);

    // div is under body, under html
    let body_box = &layout_root.children[0];
    let div_box = &body_box.children[0];

    assert_eq!(div_box.dimensions.content.width(), 400.0);
    // 800 - 400 = 400 remaining space -> 200 left, 200 right
    assert_eq!(div_box.dimensions.margin.left, 200.0);
    assert_eq!(div_box.dimensions.margin.right, 200.0);
    assert_eq!(div_box.dimensions.content.x(), 200.0);
}

#[test]
fn test_block_vertical_stacking_and_auto_height() {
    let html = r#"
        <div id="parent">
            <div style="height: 50px; margin-bottom: 10px;"></div>
            <div style="height: 100px;"></div>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let viewport_containing_block = Dimensions {
        content: Rect::new(0.0, 0.0, 500.0, 1000.0),
        ..Default::default()
    };

    layout_root.layout(&viewport_containing_block, &style_tree);

    let body_box = &layout_root.children[0];
    let parent_box = &body_box.children[0];
    let child1 = &parent_box.children[0];
    let child2 = &parent_box.children[1];

    assert_eq!(child1.dimensions.content.height(), 50.0);
    assert_eq!(child2.dimensions.content.height(), 100.0);

    // child 2 stacks below child 1's margin box: 0 + 50 + 10 = 60
    assert_eq!(child2.dimensions.content.y(), child1.dimensions.content.y() + 50.0 + 10.0);

    // Parent height expands to enclose all children: 50 + 10 + 100 = 160.0
    assert_eq!(parent_box.dimensions.content.height(), 160.0);
}

#[test]
fn test_block_overconstrained_ltr_resolution() {
    let html = r#"<div style="width: 300px; margin-left: 50px; margin-right: 50px;"></div>"#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let mut layout_root = build_layout_tree(&dom, &style_tree).unwrap();

    let viewport_cb = Dimensions {
        content: Rect::new(0.0, 0.0, 800.0, 600.0),
        ..Default::default()
    };
    layout_root.layout(&viewport_cb, &style_tree);

    let body = &layout_root.children[0];
    let div = &body.children[0];
    assert_eq!(div.dimensions.margin.left, 50.0);
    // 800 - 300 - 50 = 450.0 (margin-right absorbs underflow in LTR)
    assert_eq!(div.dimensions.margin.right, 450.0);
}
