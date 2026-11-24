//! AdBlock Plus & uBlock Origin filter list parser.

use crate::filter::{ContentType, CosmeticRule, FilterAction, FilterRule, NetworkRule};

/// Parses a single rule string line into a `FilterRule` if valid.
/// Returns `None` for comments, empty lines, and unsupported syntax.
pub fn parse_line(line: &str) -> Option<FilterRule> {
    let line = line.trim();

    // Ignore empty lines and comments
    if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
        return None;
    }

    // 1. Check for Cosmetic Filter (contains "##")
    if let Some(pos) = line.find("##") {
        let domain_part = &line[..pos];
        let selector = line[pos + 2..].trim();
        if selector.is_empty() {
            return None;
        }

        let mut domains = Vec::new();
        let mut excluded_domains = Vec::new();

        if !domain_part.is_empty() {
            for d in domain_part.split(',') {
                let d = d.trim();
                if let Some(ex) = d.strip_prefix('~') {
                    if !ex.is_empty() {
                        excluded_domains.push(ex.to_ascii_lowercase());
                    }
                } else if !d.is_empty() {
                    domains.push(d.to_ascii_lowercase());
                }
            }
        }

        return Some(FilterRule::Cosmetic(CosmeticRule {
            raw: line.to_string(),
            domains,
            excluded_domains,
            selector: selector.to_string(),
        }));
    }

    // 2. Network Rule
    let mut text = line;
    let mut action = FilterAction::Block;

    // Check for whitelist prefix "@@"
    if let Some(stripped) = text.strip_prefix("@@") {
        action = FilterAction::Allow;
        text = stripped;
    }

    let mut domains = Vec::new();
    let mut excluded_domains = Vec::new();
    let mut content_types = 0;
    let mut third_party = None;

    let pattern_part;

    // Check for options suffix "$..."
    if let Some(dollar_idx) = text.rfind('$') {
        pattern_part = &text[..dollar_idx];
        let options_str = &text[dollar_idx + 1..];

        for opt in options_str.split(',') {
            let opt = opt.trim();
            if opt.eq_ignore_ascii_case("script") {
                content_types |= ContentType::SCRIPT;
            } else if opt.eq_ignore_ascii_case("image") {
                content_types |= ContentType::IMAGE;
            } else if opt.eq_ignore_ascii_case("stylesheet") || opt.eq_ignore_ascii_case("css") {
                content_types |= ContentType::STYLESHEET;
            } else if opt.eq_ignore_ascii_case("subdocument") || opt.eq_ignore_ascii_case("frame") {
                content_types |= ContentType::SUBDOCUMENT;
            } else if opt.eq_ignore_ascii_case("xmlhttprequest") || opt.eq_ignore_ascii_case("xhr") {
                content_types |= ContentType::XMLHTTPREQUEST;
            } else if opt.eq_ignore_ascii_case("font") {
                content_types |= ContentType::FONT;
            } else if opt.eq_ignore_ascii_case("media") {
                content_types |= ContentType::MEDIA;
            } else if opt.eq_ignore_ascii_case("other") {
                content_types |= ContentType::OTHER;
            } else if opt.eq_ignore_ascii_case("third-party") || opt.eq_ignore_ascii_case("3p") {
                third_party = Some(true);
            } else if opt.eq_ignore_ascii_case("~third-party") || opt.eq_ignore_ascii_case("~3p") {
                third_party = Some(false);
            } else if let Some(domain_val) = opt.strip_prefix("domain=") {
                for d in domain_val.split('|') {
                    let d = d.trim();
                    if let Some(ex) = d.strip_prefix('~') {
                        if !ex.is_empty() {
                            excluded_domains.push(ex.to_ascii_lowercase());
                        }
                    } else if !d.is_empty() {
                        domains.push(d.to_ascii_lowercase());
                    }
                }
            }
        }
    } else {
        pattern_part = text;
    }

    if pattern_part.is_empty() {
        return None;
    }

    let mut pat = pattern_part;
    let mut is_domain_anchor = false;
    let mut is_left_anchor = false;
    let mut is_right_anchor = false;

    if let Some(stripped) = pat.strip_prefix("||") {
        is_domain_anchor = true;
        pat = stripped;
    } else if let Some(stripped) = pat.strip_prefix('|') {
        is_left_anchor = true;
        pat = stripped;
    }

    if let Some(stripped) = pat.strip_suffix('|') {
        is_right_anchor = true;
        pat = stripped;
    }

    Some(FilterRule::Network(NetworkRule {
        raw: line.to_string(),
        pattern: pat.to_ascii_lowercase(),
        is_domain_anchor,
        is_left_anchor,
        is_right_anchor,
        domains,
        excluded_domains,
        content_types,
        third_party,
        action,
    }))
}

/// Parses multi-line filter list text (e.g. EasyList contents).
pub fn parse_filter_list(content: &str) -> Vec<FilterRule> {
    content
        .lines()
        .filter_map(parse_line)
        .collect()
}
