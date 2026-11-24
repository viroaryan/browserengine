use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_navigate_data_uri_with_linked_stylesheet() {
    let mut engine = NirvanaEngine::new(400, 300);

    // Document contains a linked external stylesheet via base64 data: URI
    // "body { background-color: #00ff00; }" in base64 is "Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMGZmMDA7IH0="
    let html = r#"
        <html>
            <head>
                <link rel="stylesheet" href="data:text/css;base64,Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMGZmMDA7IH0=">
            </head>
            <body>
                <div style="height: 300px; width: 400px;">Green Page</div>
            </body>
        </html>
    "#;
    let doc_url = format!("data:text/html,{}", html.replace('\n', "").replace("  ", ""));
    let res = engine.navigate(&doc_url);
    assert!(res.is_ok());
    assert_eq!(engine.current_url(), Some(doc_url.as_str()));

    // Verify background color is green (#00ff00) from the loaded linked stylesheet
    let pixels = engine.pixels();
    assert_eq!(
        Rgba::new(pixels[0], pixels[1], pixels[2], pixels[3]),
        Rgba::new(0, 255, 0, 255)
    );
}

#[test]
fn test_engine_navigate_invalid_url_fails_gracefully() {
    let mut engine = NirvanaEngine::new(400, 300);
    let res = engine.navigate("not a valid url");
    assert!(res.is_err());
}
