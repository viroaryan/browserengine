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

#[test]
fn test_border_corners_no_double_blending() {
    let mut canvas = Canvas::new(100, 100);
    let border_color = Rgba::new(255, 0, 0, 100);
    let widths = nirvana_layout::dimensions::EdgeSizes {
        top: 10.0,
        right: 10.0,
        bottom: 10.0,
        left: 10.0,
    };

    canvas.draw_borders(
        Rect::new(20.0, 20.0, 60.0, 60.0),
        widths,
        border_color,
    );

    // Corner pixel (top-left: x=20, y=20)
    let corner_px = canvas.pixel_at(20, 20);
    // Edge midpoint pixel (top edge: x=50, y=20)
    let top_edge_px = canvas.pixel_at(50, 20);
    // Edge midpoint pixel (left edge: x=20, y=50)
    let left_edge_px = canvas.pixel_at(20, 50);

    assert_eq!(corner_px, border_color);
    assert_eq!(top_edge_px, border_color);
    assert_eq!(left_edge_px, border_color);
    assert_eq!(corner_px, top_edge_px);
}
