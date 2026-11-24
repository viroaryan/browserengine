use nirvana_font::font::Font;

#[test]
fn test_measure_text_dimensions() {
    let font = Font::default_font();
    let m1 = font.measure_text("Hello", 16.0);
    let m2 = font.measure_text("Hello World", 16.0);

    assert!(m1.width > 0.0);
    assert!(m2.width > m1.width, "Longer text must have larger advance width");
    assert!(m1.height > 0.0);
    assert!(m1.line_height >= 16.0);
}

#[test]
fn test_font_size_scaling() {
    let font = Font::default_font();
    let m16 = font.measure_text("Test", 16.0);
    let m32 = font.measure_text("Test", 32.0);

    // Twice the font size should be approximately twice the width (within 10%)
    let ratio = m32.width / m16.width;
    assert!((ratio - 2.0).abs() < 0.2, "Width should scale linearly with font size");
}

#[test]
fn test_empty_string_measurement() {
    let font = Font::default_font();
    let m = font.measure_text("", 16.0);
    assert_eq!(m.width, 0.0);
    assert!(m.line_height > 0.0);
}
