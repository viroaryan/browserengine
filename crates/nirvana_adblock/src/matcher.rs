//! Fast pattern matcher for network request filtering.

use crate::filter::{ContentType, NetworkRule};
use nirvana_url::Url;

/// Checks if character is a separator according to ABP/uBlock spec.
#[inline]
pub fn is_separator(c: char) -> bool {
    !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' && c != '%'
}

/// Matches a string against a wildcard pattern with optional `^` separators.
pub fn match_wildcard(pattern: &str, text: &str) -> bool {
    let mut pat_chars = pattern.chars().peekable();
    let mut text_chars = text.chars().peekable();

    while let Some(pc) = pat_chars.next() {
        if pc == '*' {
            let next_pat_char = loop {
                match pat_chars.peek() {
                    Some('*') => {
                        pat_chars.next();
                    }
                    other => break other.copied(),
                }
            };

            let Some(target) = next_pat_char else {
                return true; // trailing wildcard matches all remaining
            };

            let remaining_text: String = text_chars.collect();
            let mut remaining_pat: String = pat_chars.collect();

            if target == '^' {
                remaining_pat.remove(0);
                for (idx, ch) in remaining_text.char_indices() {
                    if is_separator(ch) && match_wildcard(&remaining_pat, &remaining_text[idx..]) {
                        return true;
                    }
                }
                return false;
            } else {
                for (idx, ch) in remaining_text.char_indices() {
                    if ch == target && match_wildcard(&remaining_pat, &remaining_text[idx..]) {
                        return true;
                    }
                }
                return false;
            }
        } else if pc == '^' {
            match text_chars.peek() {
                Some(&tc) if is_separator(tc) => {
                    text_chars.next();
                }
                None => {
                    // End of text satisfies separator ^
                }
                _ => return false,
            }
        } else {
            match text_chars.next() {
                Some(tc) if tc == pc => continue,
                _ => return false,
            }
        }
    }

    text_chars.next().is_none()
}

/// Extracts the base domain (eTLD+1 approximation) from a hostname.
pub fn get_base_domain(host: &str) -> &str {
    let host = host.trim().trim_end_matches('.');
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() <= 2 {
        host
    } else {
        // Simple 2-part suffix heuristic (e.g. domain.com or subdomain.domain.com -> domain.com)
        // If second to last is "co" or "com" (e.g. google.co.uk), take last 3
        let n = parts.len();
        if parts[n - 2].len() <= 3 && n >= 3 {
            let start = host.find(parts[n - 3]).unwrap_or(0);
            &host[start..]
        } else {
            let start = host.find(parts[n - 2]).unwrap_or(0);
            &host[start..]
        }
    }
}

/// Tests whether a single `NetworkRule` matches a target request.
pub fn rule_matches(
    rule: &NetworkRule,
    request_url: &Url,
    doc_origin: Option<&str>,
    content_type: ContentType,
) -> bool {
    // 1. Content type check
    if !content_type.matches(rule.content_types) {
        return false;
    }

    let req_host = request_url.host().unwrap_or("").to_ascii_lowercase();

    // 2. Third-party check
    if let Some(target_third_party) = rule.third_party
        && let Some(doc) = doc_origin
        && let Ok(doc_url) = Url::parse(doc)
    {
        let doc_host = doc_url.host().unwrap_or("").to_ascii_lowercase();
        let is_third_party = get_base_domain(&req_host) != get_base_domain(&doc_host);
        if is_third_party != target_third_party {
            return false;
        }
    }

    // 3. Document domain restrictions ($domain=...)
    if !rule.domains.is_empty() || !rule.excluded_domains.is_empty() {
        let Some(doc) = doc_origin else {
            return false;
        };
        let Ok(doc_url) = Url::parse(doc) else {
            return false;
        };
        let doc_host = doc_url.host().unwrap_or("").to_ascii_lowercase();

        // Check exclusions
        for ex in &rule.excluded_domains {
            if doc_host == *ex || doc_host.ends_with(&format!(".{ex}")) {
                return false;
            }
        }

        // Check inclusions
        if !rule.domains.is_empty() {
            let included = rule.domains.iter().any(|d| {
                doc_host == *d || doc_host.ends_with(&format!(".{d}"))
            });
            if !included {
                return false;
            }
        }
    }

    let url_str = request_url.as_str().to_ascii_lowercase();

    // 4. Pattern matching
    if rule.is_domain_anchor {
        // Pattern starts with || (e.g. ||doubleclick.net^ or ||doubleclick.net/ad.js)
        let (rule_domain, rule_path) = match rule.pattern.find(['^', '/', ':']) {
            Some(idx) => (&rule.pattern[..idx], &rule.pattern[idx..]),
            None => (rule.pattern.as_str(), ""),
        };

        let domain_match = req_host == rule_domain || req_host.ends_with(&format!(".{rule_domain}"));
        if !domain_match {
            return false;
        }

        if rule_path.is_empty() || rule_path == "^" {
            return true;
        }

        // Match remaining path part with separator/wildcards
        let path_and_query = format!(
            "{}{}",
            request_url.path(),
            request_url.query().map(|q| format!("?{q}")).unwrap_or_default()
        )
        .to_ascii_lowercase();

        if let Some(after_caret) = rule_path.strip_prefix('^') {
            if match_wildcard(after_caret, &path_and_query) {
                return true;
            }
            let trimmed = path_and_query.trim_start_matches('/');
            match_wildcard(after_caret, trimmed)
        } else {
            match_wildcard(rule_path, &path_and_query)
        }
    } else if rule.is_left_anchor && rule.is_right_anchor {
        url_str == rule.pattern
    } else if rule.is_left_anchor {
        url_str.starts_with(&rule.pattern)
    } else if rule.is_right_anchor {
        url_str.ends_with(&rule.pattern)
    } else if rule.pattern.contains('*') || rule.pattern.contains('^') {
        // Pattern contains wildcards/separators anywhere
        let pat = format!("*{}*", rule.pattern);
        match_wildcard(&pat, &url_str)
    } else {
        url_str.contains(&rule.pattern)
    }
}
