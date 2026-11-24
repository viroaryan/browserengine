//! RFC 6265 HTTP State Management Mechanism (Cookies).

use nirvana_url::Url;

/// Cookie SameSite policy attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SameSite {
    Strict,
    #[default]
    Lax,
    None,
}

/// An HTTP cookie adhering to RFC 6265.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub max_age: Option<i64>,
    pub host_only: bool,
}

impl Cookie {
    /// Determines whether this cookie matches the specified domain.
    /// Following RFC 6265 §5.1.3 & §5.3:
    /// - For host-only cookies (no explicit Domain attribute): exact host match only.
    /// - For domain cookies: identical string comparison, OR
    ///   `domain` is a suffix of `target_domain` preceded by '.'
    pub fn domain_matches(&self, target_domain: &str) -> bool {
        let cookie_domain = self.domain.to_ascii_lowercase();
        let target = target_domain.to_ascii_lowercase();

        if cookie_domain == target {
            return true;
        }

        if self.host_only {
            return false;
        }

        if target.ends_with(&cookie_domain) {
            let prefix_len = target.len() - cookie_domain.len();
            if prefix_len > 0 && target.as_bytes()[prefix_len - 1] == b'.' {
                return true;
            }
        }

        false
    }

    /// Determines whether this cookie matches the specified request path.
    /// Following RFC 6265 §5.1.4:
    /// - Identical path comparison, OR
    /// - `cookie.path` is a prefix of `target_path` and `cookie.path` ends with '/', OR
    /// - `cookie.path` is a prefix of `target_path` and the first character after prefix is '/'
    pub fn path_matches(&self, target_path: &str) -> bool {
        if self.path == target_path {
            return true;
        }

        if target_path.starts_with(&self.path) {
            if self.path.ends_with('/') {
                return true;
            }
            if target_path[self.path.len()..].starts_with('/') {
                return true;
            }
        }

        false
    }
}

/// Parses a `Set-Cookie` header value into a `Cookie`.
pub fn parse_set_cookie(header: &str, default_domain: &str, default_path: &str) -> Option<Cookie> {
    let mut parts = header.split(';');
    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let name = name.trim().to_string();
    let value = value.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut domain = default_domain.to_string();
    let mut host_only = true;
    let mut path = if default_path.is_empty() {
        "/".to_string()
    } else {
        default_path.to_string()
    };
    let mut secure = false;
    let mut http_only = false;
    let mut same_site = SameSite::Lax;
    let mut max_age = None;

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            match key.as_str() {
                "domain" => {
                    let d = val.trim_start_matches('.');
                    if !d.is_empty() {
                        domain = d.to_ascii_lowercase();
                        host_only = false;
                    }
                }
                "path" => {
                    if !val.is_empty() {
                        path = val.to_string();
                    }
                }
                "max-age" => {
                    if let Ok(secs) = val.parse::<i64>() {
                        max_age = Some(secs);
                    }
                }
                "samesite" => {
                    match val.to_ascii_lowercase().as_str() {
                        "strict" => same_site = SameSite::Strict,
                        "none" => same_site = SameSite::None,
                        _ => same_site = SameSite::Lax,
                    }
                }
                _ => {}
            }
        } else {
            let key = trimmed.to_ascii_lowercase();
            match key.as_str() {
                "secure" => secure = true,
                "httponly" => http_only = true,
                _ => {}
            }
        }
    }

    Some(Cookie {
        name,
        value,
        domain,
        path,
        secure,
        http_only,
        same_site,
        max_age,
        host_only,
    })
}

/// Sovereign in-memory cookie storage.
#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    cookies: Vec<Cookie>,
}

impl CookieJar {
    /// Creates a new empty CookieJar.
    pub fn new() -> Self {
        Self {
            cookies: Vec::new(),
        }
    }

    /// Adds a cookie directly into the jar.
    /// Overwrites existing cookie with the same `(name, domain, path)`.
    /// Evicts matching cookie if `max_age <= 0`.
    pub fn add_cookie(&mut self, cookie: Cookie) {
        if let Some(max_age) = cookie.max_age
            && max_age <= 0
        {
            self.cookies.retain(|c| {
                !(c.name.eq_ignore_ascii_case(&cookie.name)
                    && c.domain.eq_ignore_ascii_case(&cookie.domain)
                    && c.path == cookie.path)
            });
            return;
        }

        if let Some(existing) = self.cookies.iter_mut().find(|c| {
            c.name.eq_ignore_ascii_case(&cookie.name)
                && c.domain.eq_ignore_ascii_case(&cookie.domain)
                && c.path == cookie.path
        }) {
            *existing = cookie;
        } else {
            self.cookies.push(cookie);
        }
    }

    /// Stores a parsed cookie in the jar under default HTTP context.
    pub fn store(&mut self, cookie: Cookie, origin: &Url) {
        self.store_with_context(cookie, origin, true);
    }

    /// Stores a parsed cookie in the jar with access context validation.
    /// Follows RFC 6265 §5.3:
    /// - Step 5: Ignores cookies whose domain is not a suffix of the request host.
    /// - Step 11: Non-HTTP APIs (e.g. JavaScript document.cookie) cannot overwrite HttpOnly cookies.
    /// - Step 3: Evicts cookie if Max-Age <= 0.
    pub fn store_with_context(&mut self, cookie: Cookie, origin: &Url, is_http: bool) {
        let Some(origin_host) = origin.host() else {
            return;
        };

        let cookie_domain = cookie.domain.to_ascii_lowercase();
        let origin_host_lower = origin_host.to_ascii_lowercase();

        let origin_matches = if origin_host_lower == cookie_domain {
            true
        } else if origin_host_lower.ends_with(&cookie_domain) {
            let prefix_len = origin_host_lower.len() - cookie_domain.len();
            prefix_len > 0 && origin_host_lower.as_bytes()[prefix_len - 1] == b'.'
        } else {
            false
        };

        if !origin_matches {
            return;
        }

        // RFC 6265 §5.2.2: Evict matching cookie on Max-Age <= 0
        if let Some(max_age) = cookie.max_age
            && max_age <= 0
        {
            self.cookies.retain(|c| {
                !(c.name.eq_ignore_ascii_case(&cookie.name)
                    && c.domain.eq_ignore_ascii_case(&cookie.domain)
                    && c.path == cookie.path)
            });
            return;
        }

        if let Some(existing) = self.cookies.iter_mut().find(|c| {
            c.name.eq_ignore_ascii_case(&cookie.name)
                && c.domain.eq_ignore_ascii_case(&cookie.domain)
                && c.path == cookie.path
        }) {
            // RFC 6265 §5.3 Step 11: Non-HTTP APIs cannot overwrite HttpOnly cookies
            if !is_http && existing.http_only {
                return;
            }
            *existing = cookie;
        } else {
            self.cookies.push(cookie);
        }
    }

    /// Returns references to all cookies that match the target URL and access context.
    /// - If `is_http` is false (e.g. JavaScript `document.cookie`), `http_only` cookies are omitted.
    /// - If URL scheme is not `https`, `secure` cookies are omitted.
    pub fn get_cookies_for_url(&self, url: &Url, is_http: bool) -> Vec<&Cookie> {
        let Some(target_domain) = url.host() else {
            return Vec::new();
        };
        let target_path = url.path();
        let is_secure_conn = url.scheme().eq_ignore_ascii_case("https");

        self.cookies
            .iter()
            .filter(|c| {
                if !is_http && c.http_only {
                    return false;
                }
                if c.secure && !is_secure_conn {
                    return false;
                }
                if !c.domain_matches(target_domain) {
                    return false;
                }
                if !c.path_matches(target_path) {
                    return false;
                }
                true
            })
            .collect()
    }

    /// Generates the standard `Cookie: ...` header value string for network requests.
    pub fn get_cookie_header_value(&self, url: &Url, is_http: bool) -> Option<String> {
        let matching = self.get_cookies_for_url(url, is_http);
        if matching.is_empty() {
            return None;
        }

        let cookie_str = matching
            .into_iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");

        Some(cookie_str)
    }

    /// Returns all cookies currently in the jar.
    pub fn all_cookies(&self) -> &[Cookie] {
        &self.cookies
    }

    /// Returns references to all cookies that match the target URL string and access context.
    pub fn get_cookies_for_url_str(&self, url_str: &str, is_http: bool) -> Vec<&Cookie> {
        match Url::parse(url_str) {
            Ok(url) => self.get_cookies_for_url(&url, is_http),
            Err(_) => Vec::new(),
        }
    }

    /// Clears all cookies from the jar.
    pub fn clear(&mut self) {
        self.cookies.clear();
    }
}
