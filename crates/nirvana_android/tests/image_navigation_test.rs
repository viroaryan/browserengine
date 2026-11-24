use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_engine::engine::NirvanaEngine;
use nirvana_gfx::Canvas;

#[test]
fn test_engine_navigate_and_render_embedded_data_uri_image() {
    let mut engine = NirvanaEngine::new(400, 300);

    // 1. Create a 4x4 solid magenta PNG image using Nirvana Canvas
    let mut canvas = Canvas::new(4, 4);
    canvas.fill_rect(Rect::new(0.0, 0.0, 4.0, 4.0), Rgba::new(255, 0, 255, 255));
    let png_bytes = canvas.encode_png().unwrap();

    // 2. Base64 encode the PNG bytes
    let b64_png = base64_encode(&png_bytes);
    let img_src = format!("data:image/png;base64,{}", b64_png);

    // 3. Construct HTML document embedding the image with explicit 100x100 dimensions
    let html = format!(
        r#"<html><head><style>html, body {{ margin: 0; padding: 0; width: 400px; height: 300px; background-color: black; }} img {{ display: block; width: 100px; height: 100px; }}</style></head><body><img src="{}"></body></html>"#,
        img_src
    );

    let doc_url = format!("data:text/html,{}", html.replace('\n', ""));
    let res = engine.navigate(&doc_url);
    assert!(res.is_ok(), "Navigation should succeed for page with embedded image");

    // 4. Verify pixels:
    // Inside the image area (e.g. pixel 50, 50), color must be Magenta (#ff00ff)
    let p_inside = pixel_at(&engine, 50, 50);
    assert_eq!(
        p_inside,
        Rgba::new(255, 0, 255, 255),
        "Pixels inside the 100x100 image should be magenta"
    );

    // Outside the image area (e.g. pixel 150, 150), color must be Black (#000000)
    let p_outside = pixel_at(&engine, 150, 150);
    assert_eq!(
        p_outside,
        Rgba::new(0, 0, 0, 255),
        "Pixels outside the image should be black background"
    );

    // 5. Verify hit-testing on the image
    let hit = engine.hit_test(50.0, 50.0);
    assert!(hit.is_some(), "Hit-testing on image should return a target");
    assert_eq!(hit.unwrap().tag, "img");
}

#[test]
fn test_engine_navigate_broken_image_renders_placeholder_gracefully() {
    let mut engine = NirvanaEngine::new(400, 300);

    // Broken image with missing file
    let html = r#"<html><head><style>body { margin: 0; padding: 0; background-color: white; } img { display: block; width: 80px; height: 80px; }</style></head><body><img src="file:///nonexistent/missing.png" alt="Missing"></body></html>"#;
    let doc_url = format!("data:text/html,{}", html);
    let res = engine.navigate(&doc_url);
    assert!(res.is_ok(), "Navigation should not fail on broken image");

    // Check hit-testing still works on broken image placeholder
    let hit = engine.hit_test(40.0, 40.0);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().tag, "img");
}

fn pixel_at(engine: &NirvanaEngine, x: u32, y: u32) -> Rgba {
    let (w, _h) = engine.dimensions();
    let pixels = engine.pixels();
    let idx = ((y as usize) * (w as usize) + (x as usize)) * 4;
    Rgba::new(pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
