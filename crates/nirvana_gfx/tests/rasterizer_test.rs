use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_gfx::canvas::Canvas;

#[test]
fn test_canvas_solid_color_and_pixel_lookup() {
    let mut canvas = Canvas::new(100, 100);
    // Fill background with white
    canvas.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Rgba::new(255, 255, 255, 255));

    // Fill small red box in the middle (x: 20..60, y: 30..70)
    canvas.fill_rect(Rect::new(20.0, 30.0, 40.0, 40.0), Rgba::new(255, 0, 0, 255));

    assert_eq!(canvas.pixel_at(0, 0), Rgba::new(255, 255, 255, 255));
    assert_eq!(canvas.pixel_at(30, 40), Rgba::new(255, 0, 0, 255));
    assert_eq!(canvas.pixel_at(70, 70), Rgba::new(255, 255, 255, 255));
}

#[test]
fn test_canvas_alpha_blending() {
    let mut canvas = Canvas::new(10, 10);
    // Background: opaque black
    canvas.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Rgba::new(0, 0, 0, 255));

    // Foreground: 50% transparent white (rgba: 255, 255, 255, 128)
    canvas.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Rgba::new(255, 255, 255, 128));

    let pixel = canvas.pixel_at(5, 5);
    // Blended: 0.5 * 255 = ~127 or 128
    assert!((pixel.r as i32 - 128).abs() <= 2);
    assert!((pixel.g as i32 - 128).abs() <= 2);
    assert!((pixel.b as i32 - 128).abs() <= 2);
    assert_eq!(pixel.a, 255);
}

#[test]
fn test_canvas_clipping_no_panic() {
    let mut canvas = Canvas::new(50, 50);
    // Draw box far outside canvas boundaries
    canvas.fill_rect(Rect::new(-100.0, -100.0, 200.0, 200.0), Rgba::new(255, 0, 0, 255));
    assert_eq!(canvas.pixel_at(10, 10), Rgba::new(255, 0, 0, 255));
}
