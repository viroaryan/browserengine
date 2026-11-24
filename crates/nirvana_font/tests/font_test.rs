use nirvana_font::font::Font;

#[test]
fn test_default_font_loads_successfully() {
    let font = Font::default_font();
    assert!(font.has_glyph('A'));
    assert!(font.has_glyph('z'));
    assert!(font.has_glyph('0'));
    assert!(font.has_glyph(' '));
}

#[test]
fn test_invalid_font_bytes_return_error() {
    let bad_bytes = b"not a true type font";
    let result = Font::from_bytes(bad_bytes);
    assert!(result.is_err());
}
