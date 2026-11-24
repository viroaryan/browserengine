//! Font loading and TrueType table parsing wrapper.

use fontdue::{Font as InnerFont, FontSettings};
use std::sync::OnceLock;

/// Embedded default fallback font bytes.
pub static DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../assets/default_font.ttf");

static GLOBAL_DEFAULT_FONT: OnceLock<Font> = OnceLock::new();

/// A parsed TrueType/OpenType vector font.
pub struct Font {
    inner: InnerFont,
}

impl Font {
    /// Parses a font from raw TTF or OTF byte slices.
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        let inner = InnerFont::from_bytes(data, FontSettings::default())?;
        Ok(Self { inner })
    }

    /// Returns a shared reference to the embedded global default font.
    pub fn default_font() -> &'static Font {
        GLOBAL_DEFAULT_FONT.get_or_init(|| {
            Font::from_bytes(DEFAULT_FONT_BYTES)
                .expect("Failed to parse embedded default font bytes")
        })
    }

    /// Checks if a glyph exists in the font for the given character.
    pub fn has_glyph(&self, c: char) -> bool {
        self.inner.lookup_glyph_index(c) > 0
    }

    /// Returns a reference to the underlying `fontdue::Font`.
    pub fn inner(&self) -> &InnerFont {
        &self.inner
    }
}
