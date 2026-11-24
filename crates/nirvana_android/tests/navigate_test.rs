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
fn test_engine_navigate_multiple_stylesheets_cascade_order() {
    let mut engine = NirvanaEngine::new(400, 300);

    // Sheet 1: red background: "body { background-color: #ff0000; }" -> Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICNmZjAwMDA7IH0=
    // Sheet 2: blue background: "body { background-color: #0000ff; }" -> Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMDAwZmY7IH0=
    let html = r#"
        <html>
            <head>
                <link rel="stylesheet" href="data:text/css;base64,Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICNmZjAwMDA7IH0=">
                <link rel="stylesheet" href="data:text/css;base64,Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMDAwZmY7IH0=">
            </head>
            <body>
                <div style="height: 300px; width: 400px;">Cascade Page</div>
            </body>
        </html>
    "#;
    let doc_url = format!("data:text/html,{}", html.replace('\n', "").replace("  ", ""));
    let res = engine.navigate(&doc_url);
    assert!(res.is_ok());

    // In CSS cascade, Sheet 2 must win over Sheet 1, resulting in Blue (#0000ff)
    let pixels = engine.pixels();
    assert_eq!(
        Rgba::new(pixels[0], pixels[1], pixels[2], pixels[3]),
        Rgba::new(0, 0, 255, 255),
        "Later stylesheet in document order must override earlier stylesheet"
    );
}

#[test]
fn test_engine_navigate_with_failed_stylesheet_resilience() {
    let mut engine = NirvanaEngine::new(400, 300);

    // First stylesheet fails (file not found), second stylesheet succeeds with green background
    let html = r#"
        <html>
            <head>
                <link rel="stylesheet" href="file:///nonexistent/missing_styles.css">
                <link rel="stylesheet" href="data:text/css;base64,Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMGZmMDA7IH0=">
            </head>
            <body>
                <div style="height: 300px; width: 400px;">Resilient Page</div>
            </body>
        </html>
    "#;
    let doc_url = format!("data:text/html,{}", html.replace('\n', "").replace("  ", ""));
    let res = engine.navigate(&doc_url);
    assert!(res.is_ok(), "Navigation should succeed despite failed stylesheet");

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
