//! Image decoding and asset representations using pure-Rust decoders.

use std::collections::HashMap;
use std::sync::Arc;

/// Decoded 32-bit RGBA image representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// Raw RGBA8 pixel bytes (width * height * 4).
    pub rgba_pixels: Vec<u8>,
}

impl DecodedImage {
    /// Constructs a new DecodedImage from dimensions and RGBA8 buffer.
    ///
    /// # Panics
    /// Panics if `rgba_pixels.len() != (width as usize) * (height as usize) * 4`.
    pub fn new(width: u32, height: u32, rgba_pixels: Vec<u8>) -> Self {
        assert_eq!(
            rgba_pixels.len(),
            (width as usize) * (height as usize) * 4,
            "DecodedImage buffer size mismatch: expected {} bytes, got {}",
            (width as usize) * (height as usize) * 4,
            rgba_pixels.len()
        );
        Self {
            width,
            height,
            rgba_pixels,
        }
    }

    /// Returns intrinsic dimensions (width, height) in pixels.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns intrinsic aspect ratio (width / height).
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}

/// Errors that can occur during image decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    DecodeFailed(String),
    EmptyData,
    InvalidDimensions,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::DecodeFailed(msg) => write!(f, "Image decode failed: {msg}"),
            ImageError::EmptyData => write!(f, "Empty image data"),
            ImageError::InvalidDimensions => write!(f, "Invalid image dimensions"),
        }
    }
}

impl std::error::Error for ImageError {}

/// Maximum supported width and height for decoded images (8192px).
pub const MAX_IMAGE_DIMENSION: u32 = 8192;
/// Maximum uncompressed image buffer size (64 MiB).
pub const MAX_IMAGE_BUFFER_BYTES: usize = 64 * 1024 * 1024;

/// Decodes raw image bytes (PNG, JPEG) into an RGBA8 `DecodedImage`.
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    if bytes.is_empty() {
        return Err(ImageError::EmptyData);
    }

    let dyn_img = image::load_from_memory(bytes)
        .map_err(|e| ImageError::DecodeFailed(e.to_string()))?;

    let rgba = dyn_img.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(ImageError::InvalidDimensions);
    }

    let required_bytes = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if required_bytes > MAX_IMAGE_BUFFER_BYTES {
        return Err(ImageError::InvalidDimensions);
    }

    Ok(DecodedImage::new(width, height, rgba.into_raw()))
}

/// Map associating resolved image URLs to decoded image assets.
pub type ImageMap = HashMap<String, Arc<DecodedImage>>;
