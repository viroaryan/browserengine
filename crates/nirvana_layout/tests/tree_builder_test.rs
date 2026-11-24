use nirvana_html::parse_html;
use nirvana_layout::box_type::BoxType;
use nirvana_layout::tree_builder::build_layout_tree;
use nirvana_style::compute::compute_styles;

#[test]
fn test_build_layout_tree_filtering_and_hierarchy() {
    let html = r#"
        <html>
            <head><title>Ignore me</title></head>
            <body>
                <div id="container">
                    <p>Paragraph</p>
                    <span style="display: none;">Hidden</span>
                </div>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = build_layout_tree(&dom, &style_tree).expect("root layout box");

    // Root should be html (BlockNode)
    assert!(matches!(layout_root.box_type, BoxType::BlockNode(_)));

    // Head is display: none so it must NOT be in the layout tree!
    // html should only have body as a child in layout
    assert_eq!(layout_root.children.len(), 1);
    let body_box = &layout_root.children[0];
    assert!(matches!(body_box.box_type, BoxType::BlockNode(_)));

    // Body should have container
    assert_eq!(body_box.children.len(), 1);
    let container_box = &body_box.children[0];

    // Container should contain p, but NOT the hidden span!
    assert_eq!(container_box.children.len(), 1);
    let p_box = &container_box.children[0];
    assert!(matches!(p_box.box_type, BoxType::BlockNode(_)));
}

#[test]
fn test_anonymous_block_box_insertion() {
    let html = r#"
        <div>
            Some text
            <p>Block child</p>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = build_layout_tree(&dom, &style_tree).expect("root layout box");

    // Find div
    let body_box = &layout_root.children[0];
    let div_box = &body_box.children[0];

    // div contains an inline child (text) and a block child (p)
    // CSS 2.1 §9.2.1.1: inline child must be wrapped in an AnonymousBlock
    assert!(div_box.children.iter().any(|c| matches!(c.box_type, BoxType::AnonymousBlock)));
}
