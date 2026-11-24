use nirvana_core::geometry::Size;
use nirvana_css::parser::parse_css;
use nirvana_dom::{DomTree, NodeId, NodeType};
use nirvana_html::parse_html;
use nirvana_layout::tree_builder::LayoutContext;
use nirvana_layout::{layout_document, layout_document_with_context};
use nirvana_style::compute::compute_styles;

fn find_img_node(dom: &DomTree) -> NodeId {
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && dom.tag_name(node_id) == Some("img")
        {
            return node_id;
        }
        for child in dom.children(node_id) {
            stack.push(child);
        }
    }
    panic!("img node not found in DOM");
}

#[test]
fn test_image_intrinsic_sizing_with_no_css() {
    let html = "<html><body><img src=\"test.png\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);

    let img_id = find_img_node(&dom);

    let mut context = LayoutContext::default();
    context.image_dimensions.insert(img_id, (200.0, 100.0));

    let layout_root = layout_document_with_context(
        &dom,
        &style_tree,
        Size::new(800.0, 600.0),
        &context,
    )
    .unwrap();

    let body_box = &layout_root.children[0]; // <body>
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .expect("img layout box should exist");

    assert_eq!(img_box.dimensions.content.size.width, 200.0);
    assert_eq!(img_box.dimensions.content.size.height, 100.0);
}

#[test]
fn test_image_fixed_width_preserves_aspect_ratio() {
    let html = "<html><body><img src=\"test.png\" style=\"width: 400px;\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);

    let img_id = find_img_node(&dom);

    let mut context = LayoutContext::default();
    context.image_dimensions.insert(img_id, (200.0, 100.0)); // Aspect ratio 2:1

    let layout_root = layout_document_with_context(
        &dom,
        &style_tree,
        Size::new(800.0, 600.0),
        &context,
    )
    .unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    assert_eq!(img_box.dimensions.content.size.width, 400.0);
    // Height should be 400 * (100 / 200) = 200
    assert_eq!(img_box.dimensions.content.size.height, 200.0);
}

#[test]
fn test_image_fixed_height_preserves_aspect_ratio() {
    let html = "<html><body><img src=\"test.png\" style=\"height: 50px;\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);

    let img_id = find_img_node(&dom);

    let mut context = LayoutContext::default();
    context.image_dimensions.insert(img_id, (200.0, 100.0)); // Aspect ratio 2:1

    let layout_root = layout_document_with_context(
        &dom,
        &style_tree,
        Size::new(800.0, 600.0),
        &context,
    )
    .unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    // Width should be 50 * (200 / 100) = 100
    assert_eq!(img_box.dimensions.content.size.width, 100.0);
    assert_eq!(img_box.dimensions.content.size.height, 50.0);
}

#[test]
fn test_image_html_attributes_fallback() {
    let html = "<html><body><img src=\"fallback.png\" width=\"320\" height=\"240\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);

    let img_id = find_img_node(&dom);

    // No context dimensions supplied; should use HTML width/height attributes
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    assert_eq!(img_box.dimensions.content.size.width, 320.0);
    assert_eq!(img_box.dimensions.content.size.height, 240.0);
}

#[test]
fn test_image_block_centering() {
    let html = r#"
        <html>
            <body>
                <img src="logo.png" style="display: block; width: 200px; height: 100px; margin-left: auto; margin-right: auto;">
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let css = parse_css(
        "img { display: block; width: 200px; height: 100px; margin-left: auto; margin-right: auto; }",
    );
    let style_tree = compute_styles(&dom, &[ua_sheet, &css]);

    let img_id = find_img_node(&dom);

    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    assert_eq!(img_box.dimensions.content.size.width, 200.0);
    assert_eq!(img_box.dimensions.content.size.height, 100.0);
    // (800 - 200) / 2 = 300
    assert_eq!(img_box.dimensions.margin.left, 300.0);
    assert_eq!(img_box.dimensions.margin.right, 300.0);
    assert_eq!(img_box.dimensions.content.origin.x, 300.0);
}

#[test]
fn test_image_html_attributes_override_context_intrinsic_dimensions() {
    let html = "<html><body><img src=\"large.png\" width=\"300\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);

    let img_id = find_img_node(&dom);

    let mut context = LayoutContext::default();
    context.image_dimensions.insert(img_id, (1920.0, 1080.0)); // Intrinsic 16:9

    let layout_root = layout_document_with_context(
        &dom,
        &style_tree,
        Size::new(800.0, 600.0),
        &context,
    )
    .unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    assert_eq!(img_box.dimensions.content.size.width, 300.0);
    // Height scaled preserving 16:9 ratio: 300 * (1080 / 1920) = 168.75
    assert_eq!(img_box.dimensions.content.size.height, 168.75);
}

#[test]
fn test_inline_image_box_model_offsets() {
    let html = "<html><body><img src=\"small.png\" style=\"margin-left: 20px; padding-left: 10px; border-width: 5px;\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let css = parse_css("img { margin-left: 20px; padding-left: 10px; border-width: 5px; }");
    let style_tree = compute_styles(&dom, &[ua_sheet, &css]);

    let img_id = find_img_node(&dom);
    let mut context = LayoutContext::default();
    context.image_dimensions.insert(img_id, (50.0, 50.0));

    let layout_root = layout_document_with_context(
        &dom,
        &style_tree,
        Size::new(800.0, 600.0),
        &context,
    )
    .unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    // Content origin x must be container (0.0) + margin (20.0) + border (5.0) + padding (10.0) = 35.0
    assert_eq!(img_box.dimensions.content.origin.x, 35.0);
    // Border box should start at container + margin = 20.0
    assert_eq!(img_box.dimensions.border_box().origin.x, 20.0);
}

#[test]
fn test_image_block_auto_width_centering() {
    let html = "<html><body><img src=\"logo.png\" style=\"display: block; margin-left: auto; margin-right: auto;\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let css = parse_css("img { display: block; margin-left: auto; margin-right: auto; }");
    let style_tree = compute_styles(&dom, &[ua_sheet, &css]);

    let img_id = find_img_node(&dom);
    let mut context = LayoutContext::default();
    context.image_dimensions.insert(img_id, (200.0, 100.0));

    let layout_root = layout_document_with_context(
        &dom,
        &style_tree,
        Size::new(800.0, 600.0),
        &context,
    )
    .unwrap();

    let body_box = &layout_root.children[0];
    let img_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(img_id))
        .unwrap();

    assert_eq!(img_box.dimensions.content.size.width, 200.0);
    // (800 - 200) / 2 = 300
    assert_eq!(img_box.dimensions.margin.left, 300.0);
    assert_eq!(img_box.dimensions.margin.right, 300.0);
    assert_eq!(img_box.dimensions.content.origin.x, 300.0);
}
