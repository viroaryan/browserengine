use nirvana_core::geometry::Size;
use nirvana_css::parser::parse_css;
use nirvana_html::parse_html;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::compute_styles;

#[test]
fn test_full_document_layout_pipeline() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { margin: 0; padding: 0; }
                    header { height: 60px; }
                    main { width: 600px; margin: 20px auto; padding: 10px; }
                    footer { height: 40px; }
                </style>
            </head>
            <body>
                <header></header>
                <main>
                    <p style="height: 30px;"></p>
                </main>
                <footer></footer>
            </body>
        </html>
    "#;

    let dom = parse_html(html);
    let css = r#"
        body { margin: 0; padding: 0; }
        header { height: 60px; }
        main { width: 600px; margin: 20px auto; padding: 10px; }
        footer { height: 40px; }
    "#;
    let stylesheet = parse_css(css);
    let style_tree = compute_styles(&dom, &[&stylesheet]);

    let viewport = Size::new(1000.0, 800.0);
    let root_box = layout_document(&dom, &style_tree, viewport).expect("root layout box");

    // Root (html) should span viewport width (1000px)
    assert_eq!(root_box.dimensions.content.width(), 1000.0);

    let body = &root_box.children[0];
    assert_eq!(body.children.len(), 3); // header, main, footer

    let header = &body.children[0];
    assert_eq!(header.dimensions.content.height(), 60.0);
    assert_eq!(header.dimensions.content.y(), 0.0);

    let main = &body.children[1];
    assert_eq!(main.dimensions.content.width(), 600.0);
    // Auto centered in 1000px viewport: (1000 - (600 + 20 padding)) / 2 = 190.0
    assert_eq!(main.dimensions.margin.left, 190.0);
    assert_eq!(main.dimensions.margin.right, 190.0);
    // y = header height (60) + main margin-top (20) = 80.0
    assert_eq!(main.dimensions.border_box().y(), 80.0);

    let footer = &body.children[2];
    assert_eq!(footer.dimensions.content.height(), 40.0);
    // footer starts below main's margin box
    assert!(
        footer.dimensions.content.y()
            >= main.dimensions.margin_box().y() + main.dimensions.margin_box().height()
    );
}
