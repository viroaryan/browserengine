use nirvana_core::arena::Id;
use nirvana_core::color::Rgba;
use nirvana_css::parser::parse_css;
use nirvana_dom::node::Node;
use nirvana_dom::DomTree;
use nirvana_html::parse_html;
use nirvana_style::compute::compute_styles;
use nirvana_style::values::{Display, Length};

fn find_first_by_tag(dom: &DomTree, root: Id<Node>, tag: &str) -> Option<Id<Node>> {
    for child in dom.children(root) {
        if dom.tag_name(child).is_some_and(|t| t.eq_ignore_ascii_case(tag)) {
            return Some(child);
        }
        if let Some(found) = find_first_by_tag(dom, child, tag) {
            return Some(found);
        }
    }
    None
}

#[test]
fn test_user_agent_default_styles() {
    let html = "<html><head></head><body><div><p>Paragraph</p><span>Span</span></div></body></html>";
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);

    let div_id = find_first_by_tag(&dom, dom.root_id(), "div").expect("div found");
    let div_style = style_tree.get_style(div_id).unwrap();
    assert_eq!(div_style.display, Display::Block);

    let p_id = find_first_by_tag(&dom, div_id, "p").expect("p found");
    let p_style = style_tree.get_style(p_id).unwrap();
    assert_eq!(p_style.display, Display::Block);

    let span_id = find_first_by_tag(&dom, div_id, "span").expect("span found");
    let span_style = style_tree.get_style(span_id).unwrap();
    assert_eq!(span_style.display, Display::Inline);

    // Head/script/style should be Display::None
    let head_id = find_first_by_tag(&dom, dom.root_id(), "head").expect("head found");
    let head_style = style_tree.get_style(head_id).unwrap();
    assert_eq!(head_style.display, Display::None);
}

#[test]
fn test_cascade_and_specificity_override() {
    let html = r#"<div id="content" class="box alert">Text</div>"#;
    let dom = parse_html(html);
    let css = r#"
        div { color: #000000; background-color: #ffffff; }
        .box { color: #0000ff; }
        #content { color: #ff0000; }
        .alert { background-color: #ffff00; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let div_id = find_first_by_tag(&dom, dom.root_id(), "div").expect("div found");
    let style = style_tree.get_style(div_id).unwrap();

    // ID selector (#content) beats class (.box) and tag (div)
    assert_eq!(style.color, Rgba::new(255, 0, 0, 255));
    // Class selector (.alert) beats tag (div)
    assert_eq!(style.background_color, Rgba::new(255, 255, 0, 255));
}

#[test]
fn test_text_color_inheritance() {
    let html = r#"<div><p>Inherited Text</p></div>"#;
    let dom = parse_html(html);
    let css = "div { color: #ff0000; margin-left: 20px; }";
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let div_id = find_first_by_tag(&dom, dom.root_id(), "div").expect("div found");
    let p_id = find_first_by_tag(&dom, div_id, "p").expect("p found");

    let p_style = style_tree.get_style(p_id).unwrap();
    // Color inherits from div
    assert_eq!(p_style.color, Rgba::new(255, 0, 0, 255));
    // Margin does NOT inherit (defaults to 0px)
    assert_eq!(p_style.margin_left, Length::Px(0.0));
}

#[test]
fn test_source_order_tiebreaker() {
    let html = r#"<div class="box">Box</div>"#;
    let dom = parse_html(html);
    // Two rules with identical specificity (0, 1, 0)
    let css = r#"
        .box { color: #0000ff; }
        .box { color: #00ff00; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let div_id = find_first_by_tag(&dom, dom.root_id(), "div").expect("div found");
    let style = style_tree.get_style(div_id).unwrap();

    // Later rule in source order wins
    assert_eq!(style.color, Rgba::new(0, 255, 0, 255));
}
