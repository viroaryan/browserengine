use nirvana_core::geometry::Size;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_block_height_expands_to_fit_text_content() {
    let html = r#"<html><body><p>Some actual text content</p></body></html>"#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root =
        layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).expect("layout root");

    // html -> body -> p
    let body_box = &layout_root.children[0];
    let p_box = &body_box.children[0];
    assert!(
        p_box.dimensions.content.height() >= 12.0,
        "Block container height must expand to accommodate text height, was {}",
        p_box.dimensions.content.height()
    );
}
