//! Default User-Agent stylesheet according to HTML and CSS specifications.

use nirvana_css::ast::StyleSheet;
use nirvana_css::parser::parse_css;
use std::sync::OnceLock;

const USER_AGENT_CSS: &str = r#"
html, body, div, p, h1, h2, h3, h4, h5, h6, ul, ol, li,
section, article, nav, header, footer, main, form, blockquote, figure, figcaption {
    display: block;
}

head, script, style, link, meta, title {
    display: none;
}

span, a, b, i, strong, em, img, label, input, button {
    display: inline;
}
"#;

/// Returns the singleton User-Agent default stylesheet.
pub fn user_agent_stylesheet() -> &'static StyleSheet {
    static UA_SHEET: OnceLock<StyleSheet> = OnceLock::new();
    UA_SHEET.get_or_init(|| parse_css(USER_AGENT_CSS))
}
