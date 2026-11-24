//! WHATWG URL parser state machine.

use crate::error::UrlError;

/// An absolute URL representation conforming to WHATWG URL principles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    scheme: String,
    host: Option<String>,
    port: u16,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, UrlError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(UrlError::EmptyInput);
        }

        // 1. Parse scheme
        let colon_pos = input.find(':').ok_or(UrlError::MissingScheme)?;
        if colon_pos == 0 {
            return Err(UrlError::InvalidScheme);
        }

        let scheme_str = &input[..colon_pos];
        let mut scheme_chars = scheme_str.chars();
        let first = scheme_chars.next().ok_or(UrlError::InvalidScheme)?;
        if !first.is_ascii_alphabetic() {
            return Err(UrlError::InvalidScheme);
        }
        for c in scheme_chars {
            if !c.is_ascii_alphanumeric() && c != '+' && c != '-' && c != '.' {
                return Err(UrlError::InvalidScheme);
            }
        }
        let scheme = scheme_str.to_ascii_lowercase();

        let raw_remainder = &input[colon_pos + 1..];
        let is_special = matches!(scheme.as_str(), "http" | "https" | "ws" | "wss" | "ftp");

        // WHATWG: for special schemes, normalize backslashes to forward slashes
        let normalized_remainder: String;
        let mut remainder = if is_special && raw_remainder.contains('\\') {
            normalized_remainder = raw_remainder.replace('\\', "/");
            normalized_remainder.as_str()
        } else {
            raw_remainder
        };

        // 2. Parse authority (host & port) if present
        let mut host = None;
        let mut custom_port = None;

        if scheme == "file" {
            // file:///path or file://localhost/path or file:/path
            if remainder.starts_with("//") {
                remainder = &remainder[2..];
                // Check if there is a host or if it directly starts with /
                if let Some(slash_pos) = remainder.find('/') {
                    let file_host = &remainder[..slash_pos];
                    if !file_host.is_empty() && file_host != "localhost" {
                        host = Some(file_host.to_string());
                    }
                    remainder = &remainder[slash_pos..];
                }
            }
        } else if remainder.starts_with("//") {
            remainder = &remainder[2..];
            // Find end of authority: '/', '?', '#' or end of string
            let auth_end = remainder
                .find(['/', '?', '#'])
                .unwrap_or(remainder.len());
            let authority = &remainder[..auth_end];
            remainder = &remainder[auth_end..];

            // Strip optional userinfo (e.g. user:pass@)
            let host_port = match authority.rfind('@') {
                Some(at_pos) => &authority[at_pos + 1..],
                None => authority,
            };

            // Parse host and optional :port (supporting IPv6 in brackets [::1])
            if host_port.starts_with('[') {
                if let Some(bracket_end) = host_port.find(']') {
                    let h = &host_port[..=bracket_end];
                    let after = &host_port[bracket_end + 1..];
                    if let Some(port_str) = after.strip_prefix(':') {
                        let parsed_port = port_str.parse::<u16>().map_err(|_| UrlError::InvalidPort)?;
                        custom_port = Some(parsed_port);
                    } else if !after.is_empty() {
                        return Err(UrlError::InvalidHost);
                    }
                    host = Some(h.to_string());
                } else {
                    return Err(UrlError::InvalidHost);
                }
            } else if let Some(colon) = host_port.rfind(':') {
                let h = &host_port[..colon];
                let p = &host_port[colon + 1..];
                if h.is_empty() {
                    return Err(UrlError::InvalidHost);
                }
                host = Some(h.to_ascii_lowercase());
                let parsed_port = p.parse::<u16>().map_err(|_| UrlError::InvalidPort)?;
                custom_port = Some(parsed_port);
            } else {
                if host_port.is_empty() {
                    return Err(UrlError::InvalidHost);
                }
                host = Some(host_port.to_ascii_lowercase());
            }
        } else if is_special {
            return Err(UrlError::InvalidHost);
        }

        // Resolve port
        let port = custom_port.unwrap_or(match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            "ws" => 80,
            "wss" => 443,
            "ftp" => 21,
            _ => 0,
        });

        // 3. Parse path, query, fragment from remainder
        let fragment = if let Some(hash_pos) = remainder.find('#') {
            let frag = &remainder[hash_pos + 1..];
            remainder = &remainder[..hash_pos];
            Some(frag.to_string())
        } else {
            None
        };

        let query = if let Some(q_pos) = remainder.find('?') {
            let q = &remainder[q_pos + 1..];
            remainder = &remainder[..q_pos];
            Some(q.to_string())
        } else {
            None
        };

        let raw_path = if remainder.is_empty() {
            if is_special {
                "/"
            } else {
                ""
            }
        } else {
            remainder
        };

        let path = normalize_path(raw_path);

        Ok(Self {
            scheme,
            host,
            port,
            path,
            query,
            fragment,
        })
    }

    #[inline]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    #[inline]
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    #[inline]
    pub fn port(&self) -> u16 {
        self.port
    }

    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    #[inline]
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Serializes this URL into a canonical WHATWG URL string.
    pub fn as_str(&self) -> String {
        let mut s = format!("{}:", self.scheme);
        if let Some(ref h) = self.host {
            s.push_str("//");
            s.push_str(h);
            let default_port = default_port_for_scheme(&self.scheme);
            if self.port != 0 && self.port != default_port {
                s.push_str(&format!(":{}", self.port));
            }
        } else if self.scheme == "file" {
            s.push_str("//");
        }

        s.push_str(&self.path);

        if let Some(ref q) = self.query {
            s.push('?');
            s.push_str(q);
        }
        if let Some(ref f) = self.fragment {
            s.push('#');
            s.push_str(f);
        }
        s
    }

    /// Resolves a relative or absolute URL string against this base URL according to WHATWG rules.
    pub fn join(&self, input: &str) -> Result<Self, UrlError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(self.clone());
        }

        // 1. If it parses as a standalone URL with a scheme (excluding single-letter Windows drive letters)
        if let Ok(abs) = Url::parse(trimmed)
            && (abs.scheme.len() > 1 || !matches!(abs.scheme.as_str(), "c" | "d" | "e" | "f" | "p"))
        {
            return Ok(abs);
        }

        // 2. Scheme-relative URL (e.g. "//cdn.example.org/lib.js")
        if trimmed.starts_with("//") {
            let full = format!("{}:{}", self.scheme, trimmed);
            return Url::parse(&full);
        }

        // 3. Root-relative path (e.g. "/app.css")
        if trimmed.starts_with('/') {
            let authority = self.format_authority();
            let prefix = if self.host.is_some() || self.scheme == "file" {
                format!("{}://{}", self.scheme, authority)
            } else {
                format!("{}:", self.scheme)
            };
            let full = format!("{}{}", prefix, trimmed);
            return Url::parse(&full);
        }

        // 4. Query update (e.g. "?sort=asc")
        if let Some(q) = trimmed.strip_prefix('?') {
            let mut new_url = self.clone();
            new_url.query = Some(q.to_string());
            new_url.fragment = None;
            return Ok(new_url);
        }

        // 5. Fragment update (e.g. "#section")
        if let Some(f) = trimmed.strip_prefix('#') {
            let mut new_url = self.clone();
            new_url.fragment = Some(f.to_string());
            return Ok(new_url);
        }

        // 6. Path-relative URL (e.g. "style.css", "../main.css")
        let base_dir = match self.path.rfind('/') {
            Some(idx) => &self.path[..=idx],
            None => "/",
        };
        let combined = format!("{}{}", base_dir, trimmed);
        let authority = self.format_authority();
        let prefix = if self.host.is_some() || self.scheme == "file" {
            format!("{}://{}", self.scheme, authority)
        } else {
            format!("{}:", self.scheme)
        };
        let full = format!("{}{}", prefix, combined);
        Url::parse(&full)
    }

    fn format_authority(&self) -> String {
        match &self.host {
            Some(h) => {
                let default_p = default_port_for_scheme(&self.scheme);
                if self.port != 0 && self.port != default_p {
                    format!("{}:{}", h, self.port)
                } else {
                    h.clone()
                }
            }
            None => String::new(),
        }
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn default_port_for_scheme(scheme: &str) -> u16 {
    match scheme {
        "http" | "ws" => 80,
        "https" | "wss" => 443,
        "ftp" => 21,
        _ => 0,
    }
}

/// Normalizes path segments removing '.' and resolving '..' dot segments.
fn normalize_path(raw: &str) -> String {
    let starts_with_slash = raw.starts_with('/');
    let raw_segments: Vec<&str> = raw.split('/').collect();
    let ends_with_slash = raw.ends_with('/')
        || raw_segments.last().is_some_and(|s| *s == "." || *s == "..");

    let mut normalized: Vec<&str> = Vec::new();

    for seg in raw_segments {
        match seg {
            "" | "." => continue,
            ".." => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }

    let mut result = String::new();
    if starts_with_slash {
        result.push('/');
    }
    result.push_str(&normalized.join("/"));
    if ends_with_slash && !result.ends_with('/') {
        result.push('/');
    }

    if result.is_empty() && starts_with_slash {
        result.push('/');
    }

    result
}
