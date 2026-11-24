use nirvana_core::color::Rgba;
use nirvana_core::geometry::{Rect, Size};
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};
use nirvana_gfx::image::{DecodedImage, ImageMap};
use nirvana_gfx::{build_display_list_with_images, Canvas};
use nirvana_html::parse_html;
use nirvana_layout::layout_document;
use nirvana_style::compute::compute_styles;
use std::sync::Arc;

#[test]
fn test_canvas_draw_image_scaling_and_colors() {
    // 2x2 image:
    // (0,0): Red,   (1,0): Green
    // (0,1): Blue,  (1,1): Yellow
    #[rustfmt::skip]
    let pixels = vec![
        255, 0, 0, 255,       0, 255, 0, 255,
        0, 0, 255, 255,       255, 255, 0, 255,
    ];
    let img = DecodedImage::new(2, 2, pixels);

    let mut canvas = Canvas::new(100, 100);
    canvas.draw_image(Rect::new(0.0, 0.0, 100.0, 100.0), &img);

    // Sample each quadrant
    assert_eq!(canvas.pixel_at(10, 10), Rgba::new(255, 0, 0, 255), "Top-left quadrant should be red");
    assert_eq!(canvas.pixel_at(90, 10), Rgba::new(0, 255, 0, 255), "Top-right quadrant should be green");
    assert_eq!(canvas.pixel_at(10, 90), Rgba::new(0, 0, 255, 255), "Bottom-left quadrant should be blue");
    assert_eq!(canvas.pixel_at(90, 90), Rgba::new(255, 255, 0, 255), "Bottom-right quadrant should be yellow");
}

#[test]
fn test_render_display_list_draw_image_with_offset() {
    let pixels = vec![255, 128, 0, 255]; // 1x1 orange
    let img = Arc::new(DecodedImage::new(1, 1, pixels));

    let mut list = DisplayList::new();
    list.push(DisplayCommand::DrawImage {
        rect: Rect::new(50.0, 50.0, 40.0, 40.0),
        image: img,
    });

    let mut canvas = Canvas::new(100, 100);
    // Render with offset (-20, -10): target box moves to (30, 40, 40, 40)
    canvas.render_display_list_with_offset(&list, -20.0, -10.0);

    // Old location (55, 55) should now be transparent (shifted away)
    // Shifted location (35, 45) should be orange
    assert_eq!(canvas.pixel_at(35, 45), Rgba::new(255, 128, 0, 255));
    assert_eq!(canvas.pixel_at(10, 10), Rgba::new(0, 0, 0, 0));
}

#[test]
fn test_build_display_list_with_images_and_broken_fallback() {
    let html = r#"
        <html>
            <body>
                <img src="photo.png">
                <img src="broken.png" alt="Broken Photo">
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let ua_sheet = nirvana_style::user_agent::user_agent_stylesheet();
    let style_tree = compute_styles(&dom, &[ua_sheet]);
    let layout_root = layout_document(&dom, &style_tree, Size::new(800.0, 600.0)).unwrap();

    let mut images = ImageMap::new();
    let photo_asset = Arc::new(DecodedImage::new(10, 10, vec![255; 10 * 10 * 4]));
    images.insert("photo.png".to_string(), photo_asset);

    let display_list = build_display_list_with_images(&layout_root, &style_tree, &dom, &images);

    // photo.png should produce DisplayCommand::DrawImage
    let has_draw_image = display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::DrawImage { .. })
    });
    assert!(has_draw_image, "Loaded image should emit DrawImage command");

    // broken.png should produce fallback text with its alt attribute
    let has_alt_text = display_list.commands.iter().any(|cmd| {
        matches!(cmd, DisplayCommand::Text { text, .. } if text == "Broken Photo")
    });
    assert!(has_alt_text, "Missing image should render alt text");
}
