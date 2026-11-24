use nirvana_core::color::Rgba;
use nirvana_font::draw_text;
use nirvana_font::font::Font;
use nirvana_gfx::canvas::Canvas;

#[test]
fn test_rasterize_single_glyph() {
    let font = Font::default_font();
    let glyph = font.rasterize_glyph('A', 24.0);

    assert!(glyph.width > 0);
    assert!(glyph.height > 0);
    assert_eq!(glyph.alpha_mask.len(), glyph.width * glyph.height);
    // At least some pixels must have non-zero alpha coverage
    assert!(glyph.alpha_mask.iter().any(|&a| a > 0));
}

#[test]
fn test_draw_text_onto_canvas() {
    let mut canvas = Canvas::new(200, 100);
    let font = Font::default_font();
    let red = Rgba::new(255, 0, 0, 255);

    draw_text(&mut canvas, font, "Hi", 10.0, 10.0, 24.0, red);

    // Verify some red pixels were painted on the canvas
    let mut red_count = 0;
    for y in 0..100 {
        for x in 0..200 {
            let px = canvas.pixel_at(x, y);
            if px.r > 200 && px.a > 0 {
                red_count += 1;
            }
        }
    }
    assert!(
        red_count > 10,
        "Canvas must contain rendered red glyph pixels, found {}",
        red_count
    );
}
