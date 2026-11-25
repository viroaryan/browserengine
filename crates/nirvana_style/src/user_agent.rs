//! Default User-Agent stylesheet according to HTML and CSS specifications.

use nirvana_css::ast::StyleSheet;
use nirvana_css::parser::parse_css;
use std::sync::OnceLock;

const USER_AGENT_CSS: &str = r#"
html, body {
    display: block;
    margin: 0;
    font-size: 16px;
    color: #1f2937;
    background-color: #ffffff;
}

div, section, article, nav, header, footer, main, form, blockquote, figure, figcaption, address, aside {
    display: block;
}

p {
    display: block;
    font-size: 16px;
    margin: 8px 0;
}

h1 {
    display: block;
    font-size: 28px;
    font-weight: bold;
    margin: 14px 0 8px 0;
    color: #111827;
}

h2 {
    display: block;
    font-size: 22px;
    font-weight: bold;
    margin: 12px 0 6px 0;
    color: #1f2937;
}

h3 {
    display: block;
    font-size: 18px;
    font-weight: bold;
    margin: 10px 0 6px 0;
    color: #374151;
}

h4, h5, h6 {
    display: block;
    font-size: 16px;
    font-weight: bold;
    margin: 8px 0 4px 0;
}

ul, ol {
    display: block;
    margin: 8px 0;
    padding-left: 18px;
}

li {
    display: list-item;
    margin: 3px 0;
}

pre {
    display: block;
    background-color: #f3f4f6;
    padding: 8px;
    margin: 8px 0;
}

code {
    display: inline;
    background-color: #f3f4f6;
}

hr {
    display: block;
    height: 1px;
    background-color: #d1d5db;
    margin: 10px 0;
}

center {
    display: block;
    text-align: center;
}

head, script, style, noscript, link, meta, title, template, datalist, dialog, svg, option, optgroup {
    display: none;
}

[hidden] {
    display: none !important;
}

table {
    display: table;
}

tbody, thead, tfoot {
    display: table-row-group;
}

tr {
    display: table-row;
}

td, th {
    display: table-cell;
    padding: 3px 6px;
}

th {
    font-weight: bold;
}

span, label, small, sup, sub {
    display: inline;
}

small {
    font-size: 13px;
}

b, strong {
    display: inline;
    font-weight: bold;
}

i, em {
    display: inline;
    font-style: italic;
}

img {
    display: inline;
}

a {
    display: inline;
    color: #1a73e8;
    text-decoration: underline;
}

input, textarea, select {
    display: inline;
    border: 1px solid #94a3b8;
    background-color: #ffffff;
    color: #000000;
    font-size: 16px;
    padding: 4px;
    border-radius: 4px;
}

input[type="hidden"] {
    display: none !important;
}

button, input[type="submit"], input[type="button"], input[type="reset"] {
    display: inline;
    border: 1px solid #cbd5e1;
    background-color: #f1f5f9;
    color: #0f172a;
    font-size: 15px;
    font-weight: bold;
    padding: 4px 10px;
    border-radius: 6px;
}
"#;

/// Returns the singleton User-Agent default stylesheet.
pub fn user_agent_stylesheet() -> &'static StyleSheet {
    static UA_SHEET: OnceLock<StyleSheet> = OnceLock::new();
    UA_SHEET.get_or_init(|| parse_css(USER_AGENT_CSS))
}
