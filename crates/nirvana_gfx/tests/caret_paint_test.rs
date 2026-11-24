use nirvana_core::color::Rgba;
use nirvana_core::geometry::{Rect, Size};
use nirvana_gfx::builder::{build_display_list_full, FormState};
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};
use nirvana_gfx::image::ImageMap;
use nirvana_gfx::Canvas;
use nirvana_html::parse_html;
use nirvana_layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_render_unfocused_input_with_placeholder() {
    let html = "<html><body><input placeholder=\"Search...\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let form_state = FormState::default();
    let display_list = build_display_list_full(
        &layout_root,
        &style_tree,
        &dom,
        &ImageMap::new(),
        &form_state,
    );

    let has_placeholder = display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Text { text, .. } if text == "Search...")
    });
    assert!(has_placeholder, "Placeholder text should be rendered");

    let has_caret = display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Caret { .. })
    });
    assert!(!has_caret, "Unfocused input must not render a caret");
}

#[test]
fn test_render_focused_input_with_value_and_caret() {
    let html = "<html><body><input value=\"hello\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    // Find input node id
    let input_id = layout_root.children[0].children[0].node_id().unwrap();

    let mut form_state = FormState::default();
    form_state.focused_node = Some(input_id);
    form_state
        .inputs
        .insert(input_id, ("hello".to_string(), 5, None));

    let display_list = build_display_list_full(
        &layout_root,
        &style_tree,
        &dom,
        &ImageMap::new(),
        &form_state,
    );

    // Verify text
    let has_text = display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Text { text, .. } if text == "hello")
    });
    assert!(has_text, "Input value text should be rendered");

    // Verify Caret
    let caret_cmd = display_list
        .commands
        .iter()
        .find(|cmd| matches!(cmd, DisplayCommand::Caret { .. }));
    assert!(caret_cmd.is_some(), "Focused input must render a caret");
    if let Some(DisplayCommand::Caret { rect, .. }) = caret_cmd {
        assert!(rect.x() > layout_root.children[0].children[0].dimensions.content.origin.x);
        assert_eq!(rect.width(), 1.5);
    }
}

#[test]
fn test_render_password_input_masks_text() {
    let html = "<html><body><input type=\"password\" value=\"secret\"></body></html>";
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let form_state = FormState::default();
    let display_list = build_display_list_full(
        &layout_root,
        &style_tree,
        &dom,
        &ImageMap::new(),
        &form_state,
    );

    let has_masked = display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Text { text, .. } if text == "••••••")
    });
    assert!(has_masked, "Password input text should be masked as bullets");
}

#[test]
fn test_render_text_selection_and_canvas_rasterization() {
    let mut list = DisplayList::new();
    list.push(DisplayCommand::TextSelection {
        rect: Rect::new(10.0, 10.0, 50.0, 20.0),
        color: Rgba::new(0, 0, 255, 255),
    });
    list.push(DisplayCommand::Caret {
        rect: Rect::new(60.0, 10.0, 2.0, 20.0),
        color: Rgba::new(255, 0, 0, 255),
    });

    let mut canvas = Canvas::new(100, 100);
    canvas.render_display_list(&list);

    // Point inside selection: blue
    assert_eq!(canvas.pixel_at(20, 15), Rgba::new(0, 0, 255, 255));

    // Point inside caret: red
    assert_eq!(canvas.pixel_at(60, 15), Rgba::new(255, 0, 0, 255));
}
