use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_gfx::canvas::Canvas;
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};

#[test]
fn test_render_display_list_with_scroll_offset() {
    let mut canvas = Canvas::new(100, 100);
    let mut list = DisplayList::new();
    // A red box positioned at y = 50..100 in document coordinates
    list.push(DisplayCommand::SolidColor {
        rect: Rect::new(0.0, 50.0, 100.0, 50.0),
        color: Rgba::new(255, 0, 0, 255),
    });

    // When scrolled down by 50px (offset_y = -50.0), the box shifts to y = 0..50
    canvas.render_display_list_with_offset(&list, 0.0, -50.0);

    // Pixel at y = 10 should now be red
    assert_eq!(canvas.pixel_at(50, 10), Rgba::new(255, 0, 0, 255));
    // Pixel at y = 80 should be transparent black (0, 0, 0, 0)
    assert_eq!(canvas.pixel_at(50, 80), Rgba::new(0, 0, 0, 0));
}
