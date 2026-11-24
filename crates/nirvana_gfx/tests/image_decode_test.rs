use nirvana_gfx::image::{decode_image, DecodedImage, ImageError};
use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

#[test]
fn test_decode_valid_png() {
    // Generate a valid 4x4 PNG using tiny-skia
    let mut pixmap = Pixmap::new(4, 4).unwrap();
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(255, 0, 0, 255)); // Red
    pixmap.fill_rect(
        Rect::from_xywh(0.0, 0.0, 4.0, 4.0).unwrap(),
        &paint,
        Transform::identity(),
        None,
    );
    let png_bytes = pixmap.encode_png().unwrap();

    let decoded = decode_image(&png_bytes).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(decoded.dimensions(), (4, 4));
    assert_eq!(decoded.aspect_ratio(), 1.0);
    assert_eq!(decoded.rgba_pixels.len(), 4 * 4 * 4);

    // Verify first pixel is Red (255, 0, 0, 255)
    assert_eq!(decoded.rgba_pixels[0], 255);
    assert_eq!(decoded.rgba_pixels[1], 0);
    assert_eq!(decoded.rgba_pixels[2], 0);
    assert_eq!(decoded.rgba_pixels[3], 255);
}

#[test]
fn test_decode_empty_bytes_fails() {
    let err = decode_image(&[]).unwrap_err();
    assert_eq!(err, ImageError::EmptyData);
}

#[test]
fn test_decode_invalid_corrupted_bytes_fails() {
    let garbage = b"not a real png or jpeg image payload";
    let err = decode_image(garbage).unwrap_err();
    assert!(matches!(err, ImageError::DecodeFailed(_)));
}

#[test]
fn test_decoded_image_aspect_ratio() {
    let img_wide = DecodedImage::new(800, 400, vec![0; 800 * 400 * 4]);
    assert_eq!(img_wide.aspect_ratio(), 2.0);

    let img_tall = DecodedImage::new(300, 600, vec![0; 300 * 600 * 4]);
    assert_eq!(img_tall.aspect_ratio(), 0.5);
}
