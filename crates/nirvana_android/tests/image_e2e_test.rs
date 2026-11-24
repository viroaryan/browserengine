use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;
use nirvana_gfx::Canvas;

fn sample_pixel(engine: &NirvanaEngine, x: u32, y: u32) -> Rgba {
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

#[test]
fn test_image_pipeline_e2e_png_jpeg_cache_and_scrolling() {
    // 1. Generate a 4x4 Pure Green PNG
    let mut green_canvas = Canvas::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            green_canvas.put_pixel(x, y, Rgba::new(0, 255, 0, 255));
        }
    }
    let png_bytes = green_canvas.encode_png().expect("PNG encode failed");
    let png_b64 = base64_encode(&png_bytes);

    // 2. Generate a 2x2 Pure Red JPEG
    use image::codecs::jpeg::JpegEncoder;
    use image::ColorType;
    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 95);
    let raw_rgb = vec![
        255, 0, 0,    255, 0, 0,
        255, 0, 0,    255, 0, 0,
    ];
    encoder.encode(&raw_rgb, 2, 2, ColorType::Rgb8.into()).expect("JPEG encode failed");
    let jpeg_b64 = base64_encode(&jpeg_bytes);

    // 3. Compose HTML with:
    //    - PNG image #1 (width=100 height=50)
    //    - Duplicate PNG image #2 (cache reuse)
    //    - JPEG image #3 (width=50 height=50)
    //    - Broken image with alt text
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <style>
        html, body {{
            margin: 0;
            padding: 0;
            width: 400px;
            background-color: white;
        }}
        .green-img {{
            display: block;
            width: 100px;
            height: 50px;
        }}
        .red-img {{
            display: block;
            width: 50px;
            height: 50px;
        }}
    </style>
</head>
<body>
    <img class="green-img" src="data:image/png;base64,{png_b64}">
    <img class="green-img" src="data:image/png;base64,{png_b64}">
    <img class="red-img" src="data:image/jpeg;base64,{jpeg_b64}">
    <img src="data:image/png;base64,corrupt_data" alt="Corrupted Logo">
</body>
</html>"#
    );

    let mut engine = NirvanaEngine::new(400, 400);
    engine.load_html(&html, None);
    engine.render();

    // Verify first green image area: (0..100, 0..50)
    let p_green_1 = sample_pixel(&engine, 50, 25);
    assert_eq!(p_green_1.g, 255, "First PNG image pixel should be green");
    assert_eq!(p_green_1.r, 0);

    // Verify second green image area: (0..100, 50..100) (cache reused)
    let p_green_2 = sample_pixel(&engine, 50, 75);
    assert_eq!(p_green_2.g, 255, "Second PNG image (cached) pixel should be green");
    assert_eq!(p_green_2.r, 0);

    // Verify red JPEG image area: (0..50, 100..150)
    let p_red = sample_pixel(&engine, 25, 125);
    assert!(p_red.r > 200, "JPEG image pixel should be predominantly red");
    assert!(p_red.g < 50);

    // Test scrolling: scroll down 50px so that second green image moves to top (y=0)
    engine.scroll_to(0.0, 50.0);
    let p_scrolled = sample_pixel(&engine, 50, 25);
    assert_eq!(p_scrolled.g, 255, "After 50px scroll, second image should be at y=25");
}
