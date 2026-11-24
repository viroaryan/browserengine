use nirvana_engine::surface::blit_rgba_to_buffer;

#[test]
fn test_blit_rgba_with_stride_padding() {
    // 2x2 source image (each pixel is 4 bytes RGBA)
    let src_width = 2;
    let src_height = 2;
    let src_pixels = vec![
        255, 0, 0, 255,     0, 255, 0, 255,     // Row 0: Red, Green
        0, 0, 255, 255,     255, 255, 0, 255,   // Row 1: Blue, Yellow
    ];

    // Destination buffer has stride of 4 pixels (16 bytes) instead of 2 pixels (8 bytes)
    let dst_stride_bytes = 16;
    let mut dst_buffer = vec![0u8; dst_stride_bytes * src_height];

    let result = blit_rgba_to_buffer(
        &src_pixels,
        src_width,
        src_height,
        &mut dst_buffer,
        dst_stride_bytes,
    );
    assert!(result.is_ok());

    // Verify Row 0
    assert_eq!(&dst_buffer[0..4], &[255, 0, 0, 255]); // Red
    assert_eq!(&dst_buffer[4..8], &[0, 255, 0, 255]); // Green
    assert_eq!(&dst_buffer[8..16], &[0, 0, 0, 0, 0, 0, 0, 0]); // Stride padding untouched

    // Verify Row 1
    assert_eq!(&dst_buffer[16..20], &[0, 0, 255, 255]); // Blue
    assert_eq!(&dst_buffer[20..24], &[255, 255, 0, 255]); // Yellow
    assert_eq!(&dst_buffer[24..32], &[0, 0, 0, 0, 0, 0, 0, 0]); // Stride padding untouched
}

#[test]
fn test_blit_buffer_too_small_returns_error() {
    let src_pixels = vec![255, 0, 0, 255];
    let mut small_buffer = vec![0u8; 2]; // Needs 4 bytes

    let result = blit_rgba_to_buffer(&src_pixels, 1, 1, &mut small_buffer, 4);
    assert!(result.is_err());
}
