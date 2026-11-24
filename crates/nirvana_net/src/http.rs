//! HTTP/HTTPS protocol handler using ureq.

use crate::error::NetError;
use crate::handler::ProtocolHandler;
use crate::response::Response;
use nirvana_url::Url;
use std::collections::HashMap;

/// Protocol handler for `http://` and `https://` URLs.
pub struct HttpHandler;

impl ProtocolHandler for HttpHandler {
    fn can_handle(&self, scheme: &str) -> bool {
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
    }

    fn fetch(&self, url: &Url) -> Result<Response, NetError> {
        let url_str = url.as_str();

        let resp = ureq::get(&url_str)
            .header("User-Agent", "NirvanaBrowser/0.1.0 (Android; Rust Engine)")
            .call()
            .map_err(|e| NetError::NetworkFailure(e.to_string()))?;

        let status = resp.status().as_u16();

        let mut headers = HashMap::new();
        let mut mime_type = None;

        for (name, val) in resp.headers() {
            let key = name.as_str().to_ascii_lowercase();
            if let Ok(val_str) = val.to_str() {
                if key == "content-type" {
                    let mime = val_str.split(';').next().unwrap_or(val_str).trim();
                    mime_type = Some(mime.to_string());
                }
                headers.insert(key, val_str.to_string());
            }
        }

        let body = resp
            .into_body()
            .read_to_vec()
            .map_err(|e| NetError::Io(e.to_string()))?;

        Ok(Response {
            status,
            headers,
            body,
            mime_type,
        })
    }
}
