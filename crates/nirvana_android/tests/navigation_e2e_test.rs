use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_end_to_end_navigation_and_stylesheet_cascade() {
    let mut engine = NirvanaEngine::new(400, 300);

    // Page B: Blue page with "Welcome to Page B"
    // CSS: "body { background-color: #0000ff; }" -> base64: "Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMDAwZmY7IH0="
    let page_b_html = r#"
        <html>
            <head>
                <link rel="stylesheet" href="data:text/css;base64,Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICMwMDAwZmY7IH0=">
            </head>
            <body>
                <div style="height: 300px; width: 400px;">Welcome to Page B</div>
            </body>
        </html>
    "#;
    let page_b_url = format!(
        "data:text/html,{}",
        page_b_html.replace('\n', "").replace("  ", "")
    );

    // Page A: Red page with link to Page B
    // CSS: "body { background-color: #ff0000; }" -> base64: "Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICNmZjAwMDA7IH0="
    let escaped_b_url = page_b_url.replace('"', "&quot;");
    let page_a_html = format!(
        r#"
        <html>
            <head>
                <link rel="stylesheet" href="data:text/css;base64,Ym9keSB7IGJhY2tncm91bmQtY29sb3I6ICNmZjAwMDA7IH0=">
            </head>
            <body>
                <div style="height: 100px; width: 400px;">
                    <a id="link-to-b" href="{escaped_b_url}">Go to Page B</a>
                </div>
            </body>
        </html>
    "#
    );
    let page_a_url = format!(
        "data:text/html,{}",
        page_a_html.replace('\n', "").replace("  ", "")
    );

    // 1. Navigate to Page A
    assert!(engine.navigate(&page_a_url).is_ok());
    assert_eq!(engine.current_url(), Some(page_a_url.as_str()));

    // Page A must be red (#ff0000) at background (x: 200, y: 50)
    let p_a = pixel_at(&engine, 200, 50);
    assert_eq!(p_a, Rgba::new(255, 0, 0, 255));

    // 2. Hit-test link to Page B at (x: 10, y: 10)
    let hit = engine.hit_test(10.0, 10.0).expect("hit target");
    assert_eq!(hit.tag, "a");
    assert_eq!(hit.href.as_deref(), Some(page_b_url.as_str()));

    // 3. Navigate to Page B via clicked link URL
    assert!(engine.navigate(hit.href.as_deref().unwrap()).is_ok());
    assert_eq!(engine.current_url(), Some(page_b_url.as_str()));

    // Page B must now be blue (#0000ff) at background (x: 200, y: 50)
    let p_b = pixel_at(&engine, 200, 50);
    assert_eq!(p_b, Rgba::new(0, 0, 255, 255));

    // 4. Invalid scheme returns Err gracefully without panicking
    let invalid_res = engine.navigate("telnet://bbs.example.com");
    assert!(invalid_res.is_err());
    // Engine remains on Page B
    assert_eq!(engine.current_url(), Some(page_b_url.as_str()));
}

fn pixel_at(engine: &NirvanaEngine, x: u32, y: u32) -> Rgba {
    let (w, _h) = engine.dimensions();
    let idx = ((y * w + x) * 4) as usize;
    let pixels = engine.pixels();
    Rgba::new(
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    )
}
