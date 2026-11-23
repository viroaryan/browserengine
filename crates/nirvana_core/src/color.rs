//! Color representation and CSS hex color parser.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorError {
    InvalidFormat,
    InvalidHexCharacter,
}

impl std::fmt::Display for ColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid hex color format"),
            Self::InvalidHexCharacter => write!(f, "Invalid hex character in color string"),
        }
    }
}

impl std::error::Error for ColorError {}

impl Rgba {
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parses a CSS hex color string (e.g. "#fff", "#f00f", "#ffffff", "#ffffff80").
    pub fn from_hex(hex: &str) -> Result<Self, ColorError> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        let parse_nibble = |c: u8| -> Result<u8, ColorError> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(ColorError::InvalidHexCharacter),
            }
        };

        let bytes = hex.as_bytes();
        match bytes.len() {
            // #RGB
            3 => {
                let r = parse_nibble(bytes[0])?;
                let g = parse_nibble(bytes[1])?;
                let b = parse_nibble(bytes[2])?;
                Ok(Self::new(r * 17, g * 17, b * 17, 255))
            }
            // #RGBA
            4 => {
                let r = parse_nibble(bytes[0])?;
                let g = parse_nibble(bytes[1])?;
                let b = parse_nibble(bytes[2])?;
                let a = parse_nibble(bytes[3])?;
                Ok(Self::new(r * 17, g * 17, b * 17, a * 17))
            }
            // #RRGGBB
            6 => {
                let r = (parse_nibble(bytes[0])? << 4) | parse_nibble(bytes[1])?;
                let g = (parse_nibble(bytes[2])? << 4) | parse_nibble(bytes[3])?;
                let b = (parse_nibble(bytes[4])? << 4) | parse_nibble(bytes[5])?;
                Ok(Self::new(r, g, b, 255))
            }
            // #RRGGBBAA
            8 => {
                let r = (parse_nibble(bytes[0])? << 4) | parse_nibble(bytes[1])?;
                let g = (parse_nibble(bytes[2])? << 4) | parse_nibble(bytes[3])?;
                let b = (parse_nibble(bytes[4])? << 4) | parse_nibble(bytes[5])?;
                let a = (parse_nibble(bytes[6])? << 4) | parse_nibble(bytes[7])?;
                Ok(Self::new(r, g, b, a))
            }
            _ => Err(ColorError::InvalidFormat),
        }
    }
}
