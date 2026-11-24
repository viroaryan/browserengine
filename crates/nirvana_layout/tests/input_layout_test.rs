use nirvana_core::geometry::Size;
use nirvana_css::parser::parse_css;
use nirvana_dom::{DomTree, NodeId, NodeType};
use nirvana_html::parse_html;
use nirvana_layout::layout_document;
use nirvana_style::compute::compute_styles;

fn find_element_by_tag(dom: &DomTree, tag_name: &str) -> NodeId {
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && dom.tag_name(node_id).is_some_and(|t| t.eq_ignore_ascii_case(tag_name))
        {
            return node_id;
        }
        for child in dom.children(node_id) {
            stack.push(child);
        }
    }
    panic!("Element <{tag_name}> not found in DOM");
}

#[test]
fn test_input_default_intrinsic_size() {
    let html = "<html><body><input type=\"text\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let input_id = find_element_by_tag(&dom, "input");

    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();
    let body_box = &layout_root.children[0];
    let input_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(input_id))
        .expect("input layout box should exist");

    assert_eq!(input_box.dimensions.content.size.width, 160.0);
    assert_eq!(input_box.dimensions.content.size.height, 24.0);
    assert_eq!(input_box.dimensions.border.left, 1.0);
    assert_eq!(input_box.dimensions.padding.left, 4.0);
}

#[test]
fn test_input_size_attribute() {
    let html = "<html><body><input type=\"text\" size=\"10\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let input_id = find_element_by_tag(&dom, "input");

    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();
    let body_box = &layout_root.children[0];
    let input_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(input_id))
        .unwrap();

    assert_eq!(input_box.dimensions.content.size.width, 80.0);
    assert_eq!(input_box.dimensions.content.size.height, 24.0);
}

#[test]
fn test_textarea_default_and_rows_cols() {
    let html = "<html><body><textarea></textarea><textarea cols=\"30\" rows=\"5\"></textarea></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);

    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();
    let body_box = &layout_root.children[0];

    // First textarea (default)
    let ta1 = &body_box.children[0];
    assert_eq!(ta1.dimensions.content.size.width, 200.0);
    assert_eq!(ta1.dimensions.content.size.height, 60.0);

    // Second textarea (cols=30 rows=5)
    let ta2 = &body_box.children[1];
    assert_eq!(ta2.dimensions.content.size.width, 240.0);
    assert_eq!(ta2.dimensions.content.size.height, 90.0);
}

#[test]
fn test_input_css_override() {
    let html = "<html><body><input style=\"width: 320px; height: 44px;\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let css = parse_css("input { width: 320px; height: 44px; }");
    let style_tree = compute_styles(&dom, &[ua_sheet, &css]);
    let input_id = find_element_by_tag(&dom, "input");

    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();
    let body_box = &layout_root.children[0];
    let input_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(input_id))
        .unwrap();

    assert_eq!(input_box.dimensions.content.size.width, 320.0);
    assert_eq!(input_box.dimensions.content.size.height, 44.0);
}

#[test]
fn test_button_intrinsic_size() {
    let html = "<html><body><button>Click</button></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let btn_id = find_element_by_tag(&dom, "button");

    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();
    let body_box = &layout_root.children[0];
    let btn_box = body_box
        .children
        .iter()
        .find(|c| c.node_id() == Some(btn_id))
        .unwrap();

    assert_eq!(btn_box.dimensions.content.size.width, 80.0);
    assert_eq!(btn_box.dimensions.content.size.height, 24.0);
}
