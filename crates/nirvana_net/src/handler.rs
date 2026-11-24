//! Protocol handler traits and implementations for local file and data URIs.

use crate::error::NetError;
use crate::response::Response;
use nirvana_url::Url;
use std::path::Path;

/// Interface for handling specific URL schemes (e.g. `http`, `file`, `data`).
pub trait ProtocolHandler: Send + Sync + std::fmt::Debug {
    /// Returns true if this handler can serve the given URL scheme.
    fn can_handle(&self, scheme: &str) -> bool;

    /// Fetches the resource located at the given URL.
    fn fetch(&self, url: &Url) -> Result<Response, NetError>;

    /// Fetches the resource located at the given URL with custom request headers.
    fn fetch_with_headers(&self, url: &Url, _headers: &[(&str, &str)]) -> Result<Response, NetError> {
        self.fetch(url)
    }
}

/// Protocol handler for local `file://` URLs.
#[derive(Debug, Default)]
pub struct FileHandler;

impl ProtocolHandler for FileHandler {
    fn can_handle(&self, scheme: &str) -> bool {
        scheme.eq_ignore_ascii_case("file")
    }

    fn fetch(&self, url: &Url) -> Result<Response, NetError> {
        let decoded_path = percent_decode_str(url.path());
        let mut path_str = decoded_path.as_str();

        // On Windows, strip leading slash for drive-letter paths (e.g. "/C:/path" -> "C:/path")
        if path_str.len() >= 3
            && path_str.starts_with('/')
            && path_str.as_bytes()[1].is_ascii_alphabetic()
            && path_str.as_bytes()[2] == b':'
        {
            path_str = &path_str[1..];
        }

        let path = Path::new(path_str);
        let bytes = std::fs::read(path)?;
        let mime = guess_mime_type(path);

        Ok(Response::ok(bytes, Some(mime.to_string())))
    }
}

fn percent_decode_str(input: &str) -> String {
    String::from_utf8_lossy(&percent_decode(input)).into_owned()
}

/// Protocol handler for `data:` inline URIs.
#[derive(Debug, Default)]
pub struct DataUriHandler;

impl ProtocolHandler for DataUriHandler {
    fn can_handle(&self, scheme: &str) -> bool {
        scheme.eq_ignore_ascii_case("data")
    }

    fn fetch(&self, url: &Url) -> Result<Response, NetError> {
        let payload = url.path();
        let comma_idx = payload
            .find(',')
            .ok_or_else(|| NetError::DataUriDecodeError("Missing comma in data URI".to_string()))?;

        let metadata = &payload[..comma_idx];
        let raw_data = &payload[comma_idx + 1..];

        let (mime, is_base64) = if let Some(m) = metadata.strip_suffix(";base64") {
            (m, true)
        } else {
            (metadata, false)
        };

        let mime_type = if mime.is_empty() {
            "text/plain;charset=US-ASCII"
        } else {
            mime
        };

        let body = if is_base64 {
            decode_base64(raw_data)?
        } else {
            percent_decode(raw_data)
        };

        Ok(Response::ok(body, Some(mime_type.to_string())))
    }
}

fn guess_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("html" | "htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, NetError> {
    let mut clean = Vec::new();
    for b in input.bytes() {
        if !b.is_ascii_whitespace() {
            clean.push(b);
        }
    }
    while clean.last() == Some(&b'=') {
        clean.pop();
    }

    let mut output = Vec::with_capacity((clean.len() * 3) / 4);
    let mut buffer: u32 = 0;
    let mut bits_collected: u32 = 0;

    for &b in &clean {
        let val = match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(NetError::DataUriDecodeError(format!("Invalid base64 byte: {b}"))),
        };
        buffer = (buffer << 6) | val;
        bits_collected += 6;
        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push(((buffer >> bits_collected) & 0xFF) as u8);
        }
    }
    Ok(output)
}

fn percent_decode(input: &str) -> Vec<u8> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..=i + 2])
            && let Ok(val) = u8::from_str_radix(hex_str, 16)
        {
            output.push(val);
            i += 3;
            continue;
        }
        output.push(bytes[i]);
        i += 1;
    }
    output
}
