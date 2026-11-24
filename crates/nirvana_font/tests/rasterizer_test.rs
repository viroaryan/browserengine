use nirvana_font::font::Font;

#[test]
fn test_rasterize_single_glyph() {
    let font = Font::default_font();
    let glyph = font.rasterize_glyph('A', 24.0);

    assert!(glyph.width > 0);
    assert!(glyph.height > 0);
    assert!(glyph.advance_width > 0.0);
    assert_eq!(glyph.alpha_mask.len(), glyph.width * glyph.height);
    // At least some pixels must have non-zero alpha coverage
    assert!(glyph.alpha_mask.iter().any(|&a| a > 0));
}

#[test]
fn test_rasterize_whitespace_glyph() {
    let font = Font::default_font();
    let glyph = font.rasterize_glyph(' ', 24.0);

    // Whitespace has zero bitmap area but positive advance width
    assert!(glyph.advance_width > 0.0);
    assert_eq!(glyph.width, 0);
    assert_eq!(glyph.height, 0);
}
