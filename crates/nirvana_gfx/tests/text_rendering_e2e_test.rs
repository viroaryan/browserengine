use nirvana_core::geometry::Size;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_html_text_renders_actual_glyphs_into_canvas() {
    let html = r#"
        <div style="background-color: #ffffff; padding: 20px;">
            <p style="color: #ff0000;">Hello Nirvana</p>
        </div>
    "#;
    let dom = parse_html(html);
    let style_tree = compute_styles(&dom, &[]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(400.0, 200.0)).unwrap();
    let display_list = build_display_list(&layout_root, &style_tree, &dom);

    let mut canvas = Canvas::new(400, 200);
    canvas.render_display_list(&display_list);

    // Verify canvas contains anti-aliased red text pixels
    let mut red_pixel_count = 0;
    for y in 0..200 {
        for x in 0..400 {
            let px = canvas.pixel_at(x, y);
            if px.r > 200 && px.g < 50 && px.b < 50 && px.a > 0 {
                red_pixel_count += 1;
            }
        }
    }

    assert!(
        red_pixel_count > 50,
        "Expected multiple red glyph pixels from rendered text, found {}",
        red_pixel_count
    );
}
