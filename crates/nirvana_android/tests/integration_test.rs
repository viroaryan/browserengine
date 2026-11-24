use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_end_to_end_engine_pipeline() {
    let mut engine = NirvanaEngine::new(400, 300);

    let html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { margin: 0; background-color: #ffffff; }
                    header { height: 50px; background-color: #0000ff; }
                    main { width: 200px; height: 100px; margin: 20px auto; background-color: #00ff00; }
                </style>
            </head>
            <body>
                <header></header>
                <main></main>
            </body>
        </html>
    "#;
    let css = r#"
        body { margin: 0; background-color: #ffffff; }
        header { height: 50px; background-color: #0000ff; }
        main { width: 200px; height: 100px; margin: 20px auto; background-color: #00ff00; }
    "#;

    engine.load_html(html, Some(css));
    engine.render();

    let pixels = engine.pixels();
    assert_eq!(pixels.len(), 400 * 300 * 4);

    // Verify header pixel at (x: 200, y: 25) is blue (#0000ff: R=0, G=0, B=255, A=255)
    let header_idx = (25 * 400 + 200) * 4;
    assert_eq!(pixels[header_idx], 0);
    assert_eq!(pixels[header_idx + 1], 0);
    assert_eq!(pixels[header_idx + 2], 255);
    assert_eq!(pixels[header_idx + 3], 255);

    // Verify main centered box (width: 200 in 400 container -> x: 100..300, y: 70..170) is green (#00ff00: R=0, G=255, B=0, A=255)
    let main_idx = (120 * 400 + 200) * 4;
    assert_eq!(pixels[main_idx], 0);
    assert_eq!(pixels[main_idx + 1], 255);
    assert_eq!(pixels[main_idx + 2], 0);
    assert_eq!(pixels[main_idx + 3], 255);

    // Verify outside margin area (x: 50, y: 120) is body white (#ffffff: R=255, G=255, B=255, A=255)
    let margin_idx = (120 * 400 + 50) * 4;
    assert_eq!(pixels[margin_idx], 255);
    assert_eq!(pixels[margin_idx + 1], 255);
    assert_eq!(pixels[margin_idx + 2], 255);
    assert_eq!(pixels[margin_idx + 3], 255);

    // Test resize re-rendering
    engine.resize(600, 400);
    assert_eq!(engine.dimensions(), (600, 400));
    assert_eq!(engine.pixels().len(), 600 * 400 * 4);
}
