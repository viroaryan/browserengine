use nirvana_core::color::Rgba;
use nirvana_core::geometry::{Point, Rect};
use nirvana_gfx::display_list::{DisplayCommand, DisplayList};
use nirvana_layout::dimensions::EdgeSizes;

#[test]
fn test_display_list_commands() {
    let mut list = DisplayList::new();
    list.push(DisplayCommand::SolidColor {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        color: Rgba::new(255, 0, 0, 255),
    });
    list.push(DisplayCommand::Border {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        widths: EdgeSizes::new(2.0, 2.0, 2.0, 2.0),
        color: Rgba::new(0, 0, 0, 255),
    });
    list.push(DisplayCommand::Text {
        text: "Hello Nirvana".into(),
        origin: Point::new(10.0, 30.0),
        color: Rgba::new(255, 255, 255, 255),
        font_size: 16.0,
    });

    assert_eq!(list.commands.len(), 3);
    assert!(matches!(&list.commands[0], DisplayCommand::SolidColor { .. }));
    assert!(matches!(&list.commands[1], DisplayCommand::Border { .. }));
    assert!(matches!(&list.commands[2], DisplayCommand::Text { .. }));
}
