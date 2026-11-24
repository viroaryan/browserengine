use nirvana_core::geometry::Rect;
use nirvana_layout::dimensions::{Dimensions, EdgeSizes};

#[test]
fn test_box_model_areas() {
    let d = Dimensions {
        content: Rect::new(50.0, 50.0, 200.0, 100.0),
        padding: EdgeSizes::new(10.0, 10.0, 10.0, 10.0),
        border: EdgeSizes::new(2.0, 2.0, 2.0, 2.0),
        margin: EdgeSizes::new(20.0, 15.0, 20.0, 15.0),
    };

    // Padding box: content expanded by padding
    let p_box = d.padding_box();
    assert_eq!(p_box, Rect::new(40.0, 40.0, 220.0, 120.0));

    // Border box: padding box expanded by border
    let b_box = d.border_box();
    assert_eq!(b_box, Rect::new(38.0, 38.0, 224.0, 124.0));

    // Margin box: border box expanded by margin
    let m_box = d.margin_box();
    assert_eq!(m_box, Rect::new(23.0, 18.0, 254.0, 164.0));
}
