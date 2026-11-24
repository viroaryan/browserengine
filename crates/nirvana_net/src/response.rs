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
}
