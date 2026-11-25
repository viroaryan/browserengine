//! Resource response data structure.

use std::collections::HashMap;

/// An HTTP or protocol resource response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status code or protocol equivalent (200 for OK).
    pub status: u16,
    /// Response headers (case-insensitive keys converted to lowercase).
    pub headers: HashMap<String, String>,
    /// Preserved ordered list of all raw header pairs (useful for multiple Set-Cookie headers).
    pub raw_headers: Vec<(String, String)>,
    /// Raw payload bytes.
    pub body: Vec<u8>,
    /// Extracted or inferred MIME type (e.g. "text/html", "text/css").
    pub mime_type: Option<String>,
    /// Final resolved URL after HTTP redirects, if available.
    pub final_url: Option<String>,
}

impl Response {
    /// Creates a new successful (200 OK) Response.
    pub fn ok(body: Vec<u8>, mime_type: Option<String>) -> Self {
        let mut headers = HashMap::new();
        let mut raw_headers = Vec::new();
        if let Some(ref m) = mime_type {
            headers.insert("content-type".to_string(), m.clone());
            raw_headers.push(("content-type".to_string(), m.clone()));
        }
        Self {
            status: 200,
            headers,
            raw_headers,
            body,
            mime_type,
            final_url: None,
        }
    }

    /// Returns all header values for a case-insensitive header name.
    pub fn get_all_headers(&self, name: &str) -> Vec<&str> {
        let name_lower = name.to_ascii_lowercase();
        self.raw_headers
            .iter()
            .filter(|(k, _)| k.to_ascii_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Converts the response body into a UTF-8 string slice reference.
    pub fn text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    /// Decodes the response body cleanly into a `String`, handling UTF-8 BOM,
    /// explicit ISO-8859-1 / Windows-1252 Content-Type charsets, and lossy UTF-8 fallback.
    pub fn decoded_text(&self) -> String {
        let bytes = self.body.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&self.body);

        let declared_latin1 = self
            .headers
            .get("content-type")
            .map(|ct| {
                let lower = ct.to_ascii_lowercase();
                lower.contains("iso-8859-1") || lower.contains("windows-1252") || lower.contains("latin1")
            })
            .unwrap_or(false);

        if declared_latin1 && std::str::from_utf8(bytes).is_err() {
            return decode_windows_1252(bytes);
        }

        match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                if declared_latin1 {
                    decode_windows_1252(bytes)
                } else {
                    String::from_utf8_lossy(bytes).into_owned()
                }
            }
        }
    }
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let ch = match b {
            0x80 => '\u{20AC}',
            0x82 => '\u{201A}',
            0x83 => '\u{0192}',
            0x84 => '\u{201E}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02C6}',
            0x89 => '\u{2030}',
            0x8A => '\u{0160}',
            0x8B => '\u{2039}',
            0x8C => '\u{0152}',
            0x8E => '\u{017D}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201C}',
            0x94 => '\u{201D}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02DC}',
            0x99 => '\u{2122}',
            0x9A => '\u{0161}',
            0x9B => '\u{203A}',
            0x9C => '\u{0153}',
            0x9E => '\u{017E}',
            0x9F => '\u{0178}',
            other => char::from(other),
        };
        out.push(ch);
    }
    out
}
