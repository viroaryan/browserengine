use nirvana_core::color::Rgba;
use nirvana_core::geometry::Size;
use nirvana_css::parser::parse_css;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_end_to_end_html_css_to_pixel_buffer() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { margin: 0; background-color: #ffffff; }
                    header { height: 50px; background-color: #0000ff; }
                    main { width: 200px; height: 100px; margin: 20px auto; background-color: #ff0000; }
                </style>
            </head>
            <body>
                <header></header>
                <main></main>
            </body>
        </html>
    "#;
    let dom = parse_html(html);
    let css = r#"
        body { margin: 0; background-color: #ffffff; }
        header { height: 50px; background-color: #0000ff; }
        main { width: 200px; height: 100px; margin: 20px auto; background-color: #ff0000; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let viewport = Size::new(400.0, 300.0);
    let layout_root = layout_document(&dom, &style_tree, viewport).expect("layout root");
    let display_list = build_display_list(&layout_root, &style_tree, &dom);

    let mut canvas = Canvas::new(400, 300);
    canvas.render_display_list(&display_list);

    // Verify header pixel (x: 200, y: 25) is blue (#0000ff)
    assert_eq!(canvas.pixel_at(200, 25), Rgba::new(0, 0, 255, 255));

    // Verify main centered box (width: 200, in 400 viewport -> x: 100..300, y: 70..170) is red (#ff0000)
    assert_eq!(canvas.pixel_at(200, 120), Rgba::new(255, 0, 0, 255));

    // Verify outside margin area (x: 50, y: 120) is body white (#ffffff)
    assert_eq!(canvas.pixel_at(50, 120), Rgba::new(255, 255, 255, 255));

    // Verify PNG encoding works
    let png_bytes = canvas.encode_png().expect("PNG encoded");
    assert!(!png_bytes.is_empty());
    assert_eq!(&png_bytes[0..8], b"\x89PNG\r\n\x1a\n");
}
