use nirvana_core::geometry::{Point, Rect, Size};
use nirvana_core::color::Rgba;
use nirvana_core::arena::Arena;

#[test]
fn test_geometry_containment_and_intersection() {
    let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert!(r1.contains_point(Point::new(50.0, 50.0)));
    assert!(!r1.contains_point(Point::new(150.0, 50.0)));

    let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
    assert!(r1.intersects(&r2));

    let r3 = Rect::new(200.0, 200.0, 50.0, 50.0);
    assert!(!r1.intersects(&r3));

    let u = r1.union(&r2);
    assert_eq!(u, Rect::new(0.0, 0.0, 150.0, 150.0));

    let s = Size::new(100.0, 50.0);
    assert_eq!(s.width, 100.0);
    assert_eq!(s.height, 50.0);
}

#[test]
fn test_color_hex_parsing() {
    assert_eq!(Rgba::from_hex("#ff0000").unwrap(), Rgba::new(255, 0, 0, 255));
    assert_eq!(Rgba::from_hex("#00ff007f").unwrap(), Rgba::new(0, 255, 0, 127));
    assert_eq!(Rgba::from_hex("#f00").unwrap(), Rgba::new(255, 0, 0, 255));
    assert!(Rgba::from_hex("#xyz").is_err());
    assert!(Rgba::from_hex("#12").is_err());
    assert!(Rgba::from_hex("#12345").is_err());
}

#[test]
fn test_arena_storage() {
    let mut arena: Arena<String> = Arena::new();
    let id1 = arena.alloc("Node1".to_string());
    let id2 = arena.alloc("Node2".to_string());
    assert_eq!(arena.get(id1), Some(&"Node1".to_string()));
    assert_eq!(arena.get(id2), Some(&"Node2".to_string()));
    assert_eq!(arena.len(), 2);
    assert!(!arena.is_empty());

    if let Some(val) = arena.get_mut(id1) {
        *val = "UpdatedNode1".to_string();
    }
    assert_eq!(arena.get(id1), Some(&"UpdatedNode1".to_string()));
}
