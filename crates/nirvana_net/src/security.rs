//! Web Security primitives: Origin, Same-Origin Policy (SOP), Mixed Content detection, and CSP.

use nirvana_url::Url;
use std::collections::HashMap;

/// An origin per the WHATWG HTML standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Tuple {
        scheme: String,
        host: String,
        port: u16,
    },
    Opaque(u64),
}

static OPAQUE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Origin {
    /// Constructs an `Origin` from a parsed `Url`.
    pub fn from_url(url: &Url) -> Self {
        let scheme = url.scheme().to_ascii_lowercase();
        if scheme == "data" || scheme == "javascript" || scheme == "about" {
            let id = OPAQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Origin::Opaque(id);
        }

        let Some(host) = url.host() else {
            let id = OPAQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Origin::Opaque(id);
        };

        let port = url.port();

        Origin::Tuple {
            scheme,
            host: host.to_ascii_lowercase(),
            port,
        }
    }

    /// Returns whether this origin is opaque.
    pub fn is_opaque(&self) -> bool {
        matches!(self, Origin::Opaque(_))
    }

    /// Returns the scheme of the origin, if not opaque.
    pub fn scheme(&self) -> &str {
        match self {
            Origin::Tuple { scheme, .. } => scheme,
            Origin::Opaque(_) => "opaque",
        }
    }

    /// Returns the host of the origin, if not opaque.
    pub fn host(&self) -> Option<&str> {
        match self {
            Origin::Tuple { host, .. } => Some(host),
            Origin::Opaque(_) => None,
        }
    }

    /// Returns the port of the origin, if not opaque.
    pub fn port(&self) -> Option<u16> {
        match self {
            Origin::Tuple { port, .. } => Some(*port),
            Origin::Opaque(_) => None,
        }
    }

    /// Determines whether two origins are identical according to Same-Origin Policy.
    pub fn is_same_origin(&self, other: &Origin) -> bool {
        match (self, other) {
            (
                Origin::Tuple {
                    scheme: s1,
                    host: h1,
                    port: p1,
                },
                Origin::Tuple {
                    scheme: s2,
                    host: h2,
                    port: p2,
                },
            ) => s1 == s2 && h1 == h2 && p1 == p2,
            _ => false, // Opaque origins never match
        }
    }

    /// Serializes the origin as a string (e.g. `https://example.com` or `https://example.com:8443`).
    pub fn as_string(&self) -> String {
        match self {
            Origin::Tuple { scheme, host, port } => {
                let is_default_port = match (scheme.as_str(), *port) {
                    ("http", 80) => true,
                    ("https", 443) => true,
                    _ => false,
                };
                if is_default_port || *port == 0 {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                }
            }
            Origin::Opaque(id) => format!("null:{id}"),
        }
    }
}

/// Detects whether a subresource request constitutes mixed content.
/// If document origin is HTTPS and subresource is HTTP, returns `true`.
pub fn is_mixed_content(doc_origin: &Origin, subresource_url: &Url) -> bool {
    if !doc_origin.scheme().eq_ignore_ascii_case("https") {
        return false;
    }
    let sub_scheme = subresource_url.scheme().to_ascii_lowercase();
    sub_scheme == "http" || sub_scheme == "ws"
}

/// Content-Security-Policy (CSP) Directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CspDirective {
    DefaultSrc,
    ScriptSrc,
    StyleSrc,
    ImgSrc,
    ConnectSrc,
}

/// A parsed Content-Security-Policy header.
#[derive(Debug, Clone, Default)]
pub struct CspPolicy {
    directives: HashMap<CspDirective, Vec<String>>,
}

impl CspPolicy {
    /// Parses a `Content-Security-Policy` header value.
    pub fn parse(header: &str) -> Option<Self> {
        let mut directives = HashMap::new();
        for directive_str in header.split(';') {
            let mut tokens = directive_str.split_whitespace();
            let Some(name) = tokens.next() else {
                continue;
            };
            let dir = match name.to_ascii_lowercase().as_str() {
                "default-src" => CspDirective::DefaultSrc,
                "script-src" => CspDirective::ScriptSrc,
                "style-src" => CspDirective::StyleSrc,
                "img-src" => CspDirective::ImgSrc,
                "connect-src" => CspDirective::ConnectSrc,
                _ => continue,
            };
            let sources: Vec<String> = tokens.map(|s| s.to_string()).collect();
            directives.insert(dir, sources);
        }
        Some(Self { directives })
    }

    /// Evaluates whether a resource URL is permitted by the policy for given directive and document origin.
    pub fn is_allowed(&self, directive: CspDirective, url: &Url, doc_origin: &Origin) -> bool {
        let sources = self
            .directives
            .get(&directive)
            .or_else(|| self.directives.get(&CspDirective::DefaultSrc));

        let Some(sources) = sources else {
            return true; // No policy restriction for this directive
        };

        let sub_scheme = url.scheme();
        let sub_host = url.host().unwrap_or("");

        for source in sources {
            let src = source.trim();
            if src == "*" {
                return true;
            }
            if src.eq_ignore_ascii_case("'self'") {
                let sub_origin = Origin::from_url(url);
                if doc_origin.is_same_origin(&sub_origin) {
                    return true;
                }
            }
            if src.ends_with(':') {
                let expected_scheme = &src[..src.len() - 1];
                if sub_scheme.eq_ignore_ascii_case(expected_scheme) {
                    return true;
                }
            }
            if let Ok(allowed_url) = Url::parse(src) {
                if let Some(allowed_host) = allowed_url.host() {
                    if sub_host.eq_ignore_ascii_case(allowed_host)
                        && sub_scheme.eq_ignore_ascii_case(allowed_url.scheme())
                    {
                        return true;
                    }
                }
            }
        }

        false
    }
}
