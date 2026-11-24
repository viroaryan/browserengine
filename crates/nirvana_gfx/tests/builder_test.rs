use nirvana_core::color::Rgba;
use nirvana_core::geometry::Size;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::display_list::DisplayCommand;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_build_display_list_painting_order() {
    let html = r#"
        <div style="background-color: #ff0000; border: 2px; padding: 10px;">
            <p style="background-color: #00ff00;">Text content</p>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let display_list = build_display_list(&layout_root, &style_tree, &dom);

    // Filter commands by color
    let red = Rgba::new(255, 0, 0, 255);
    let green = Rgba::new(0, 255, 0, 255);

    // Parent background must precede child background in the display list
    let red_idx = display_list
        .commands
        .iter()
        .position(|cmd| matches!(cmd, DisplayCommand::SolidColor { color, .. } if color == &red))
        .expect("parent red background emitted");

    let green_idx = display_list
        .commands
        .iter()
        .position(|cmd| matches!(cmd, DisplayCommand::SolidColor { color, .. } if color == &green))
        .expect("child green background emitted");

    assert!(
        red_idx < green_idx,
        "Parent background must be painted before child background"
    );

    // Text content must be emitted
    assert!(display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Text { text, .. } if text.contains("Text content"))
    }));
}
