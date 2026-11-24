//! Cosmetic filter engine for generating element-hiding CSS.

use crate::filter::CosmeticRule;
use nirvana_url::Url;

/// Generates an injected CSS stylesheet hiding all ad elements matching cosmetic rules.
pub fn generate_cosmetic_stylesheet(rules: &[CosmeticRule], document_url: Option<&str>) -> Option<String> {
    let doc_host = document_url
        .and_then(|u| Url::parse(u).ok())
        .and_then(|u| u.host().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_default();

    let mut selectors = Vec::new();

    for rule in rules {
        // Exclusions check (~domain)
        if !doc_host.is_empty() {
            let is_excluded = rule.excluded_domains.iter().any(|ex| {
                doc_host == *ex || doc_host.ends_with(&format!(".{ex}"))
            });
            if is_excluded {
                continue;
            }
        }

        // Inclusions check
        if !rule.domains.is_empty() {
            if doc_host.is_empty() {
                continue;
            }
            let is_included = rule.domains.iter().any(|d| {
                doc_host == *d || doc_host.ends_with(&format!(".{d}"))
            });
            if !is_included {
                continue;
            }
        }

        selectors.push(rule.selector.as_str());
    }

    if selectors.is_empty() {
        None
    } else {
        Some(format!(
            "{} {{ display: none !important; }}",
            selectors.join(", ")
        ))
    }
}
