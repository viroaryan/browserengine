//! Surface buffer blitting and Android Native Window integration.

/// Blits an RGBA byte slice into a destination pixel buffer supporting row stride / pitch.
///
/// # Arguments
/// - `src_pixels`: Contiguous RGBA source pixels (`src_width * src_height * 4` bytes).
/// - `src_width`: Width of source image in pixels.
/// - `src_height`: Height of source image in pixels.
/// - `dst_buffer`: Target buffer (e.g. locked `ANativeWindow_Buffer` memory).
/// - `dst_stride_bytes`: Number of bytes per row in destination buffer (`stride * 4`).
pub fn blit_rgba_to_buffer(
    src_pixels: &[u8],
    src_width: usize,
    src_height: usize,
    dst_buffer: &mut [u8],
    dst_stride_bytes: usize,
) -> Result<(), &'static str> {
    let src_row_bytes = src_width.checked_mul(4).ok_or("Source width overflow")?;
    let expected_src_len = src_row_bytes
        .checked_mul(src_height)
        .ok_or("Source size overflow")?;

    if src_pixels.len() < expected_src_len {
        return Err("Source pixel buffer is smaller than width * height * 4");
    }

    if dst_stride_bytes < src_row_bytes {
        return Err("Destination stride is smaller than row width in bytes");
    }

    let min_dst_len = if src_height == 0 {
        0
    } else {
        (src_height - 1)
            .checked_mul(dst_stride_bytes)
            .and_then(|offset| offset.checked_add(src_row_bytes))
            .ok_or("Destination size overflow")?
    };

    if dst_buffer.len() < min_dst_len {
        return Err("Destination buffer too small to hold blitted image");
    }

    if src_row_bytes == dst_stride_bytes {
        // Fast path: contiguous memory copy
        dst_buffer[..expected_src_len].copy_from_slice(&src_pixels[..expected_src_len]);
    } else {
        // Stride-aware row-by-row blit
        for y in 0..src_height {
            let src_start = y * src_row_bytes;
            let src_end = src_start + src_row_bytes;

            let dst_start = y * dst_stride_bytes;
            let dst_end = dst_start + src_row_bytes;

            dst_buffer[dst_start..dst_end].copy_from_slice(&src_pixels[src_start..src_end]);
        }
    }

    Ok(())
}
