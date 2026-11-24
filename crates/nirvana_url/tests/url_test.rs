use nirvana_url::{Url, UrlError};

#[test]
fn test_url_standard_http() {
    let url = Url::parse("http://example.com/index.html?search=nirvana#section1").unwrap();
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host(), Some("example.com"));
    assert_eq!(url.port(), 80);
    assert_eq!(url.path(), "/index.html");
    assert_eq!(url.query(), Some("search=nirvana"));
    assert_eq!(url.fragment(), Some("section1"));
}

#[test]
fn test_url_custom_port() {
    let url = Url::parse("https://example.com:8443/api").unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host(), Some("example.com"));
    assert_eq!(url.port(), 8443);
    assert_eq!(url.path(), "/api");
}

#[test]
fn test_url_file_scheme() {
    let url = Url::parse("file:///sdcard/Download/test.html").unwrap();
    assert_eq!(url.scheme(), "file");
    assert_eq!(url.path(), "/sdcard/Download/test.html");
    assert_eq!(url.port(), 0);
}

#[test]
fn test_url_path_dot_normalization() {
    let url = Url::parse("http://example.com/a/b/../c").unwrap();
    assert_eq!(url.path(), "/a/c");

    let url2 = Url::parse("http://example.com/a/./b/../../c").unwrap();
    assert_eq!(url2.path(), "/c");
}

#[test]
fn test_url_default_ports() {
    let http = Url::parse("http://localhost/").unwrap();
    assert_eq!(http.port(), 80);

    let https = Url::parse("https://localhost/").unwrap();
    assert_eq!(https.port(), 443);
}

#[test]
fn test_url_invalid_input() {
    assert!(matches!(Url::parse("not-a-url"), Err(UrlError::MissingScheme)));
    assert!(matches!(Url::parse("://bad"), Err(UrlError::InvalidScheme)));
}

#[test]
fn test_url_directory_dot_segment_trailing_slash_and_backslash() {
    let url1 = Url::parse("http://example.com/a/b/..").unwrap();
    assert_eq!(url1.path(), "/a/");

    let url2 = Url::parse("http://example.com/a/b/.").unwrap();
    assert_eq!(url2.path(), "/a/b/");

    let url3 = Url::parse(r"http://example.com\path\file.html").unwrap();
    assert_eq!(url3.host(), Some("example.com"));
    assert_eq!(url3.path(), "/path/file.html");
}

#[test]
fn test_url_ipv6_host() {
    let url1 = Url::parse("http://[::1]/").unwrap();
    assert_eq!(url1.host(), Some("[::1]"));
    assert_eq!(url1.port(), 80);

    let url2 = Url::parse("http://[2001:db8::1]:8080/test").unwrap();
    assert_eq!(url2.host(), Some("[2001:db8::1]"));
    assert_eq!(url2.port(), 8080);
}

#[test]
fn test_url_opaque_scheme_no_dot_segment_normalization() {
    let data_url = Url::parse("data:text/plain,a//b/../c").unwrap();
    assert_eq!(data_url.scheme(), "data");
    assert_eq!(data_url.path(), "text/plain,a//b/../c");

    let base64_url = Url::parse("data:text/html;base64,PGh0bWw+..//123").unwrap();
    assert_eq!(base64_url.scheme(), "data");
    assert_eq!(base64_url.path(), "text/html;base64,PGh0bWw+..//123");
}

#[test]
fn test_url_join_query_with_fragment() {
    let base = Url::parse("https://example.com/items/view.html").unwrap();
    let joined = base.join("?sort=asc#results").unwrap();
    assert_eq!(joined.path(), "/items/view.html");
    assert_eq!(joined.query(), Some("sort=asc"));
    assert_eq!(joined.fragment(), Some("results"));
    assert_eq!(joined.as_str(), "https://example.com/items/view.html?sort=asc#results");

    // Joining query without fragment should clear previous fragment
    let base_with_frag = Url::parse("https://example.com/items?filter=all#old").unwrap();
    let joined2 = base_with_frag.join("?filter=new").unwrap();
    assert_eq!(joined2.query(), Some("filter=new"));
    assert_eq!(joined2.fragment(), None);
}

#[test]
fn test_url_join_drive_letter_not_scheme() {
    let base = Url::parse("https://example.com/app/").unwrap();
    let abs_http = base.join("https://other.org/style.css").unwrap();
    assert_eq!(abs_http.as_str(), "https://other.org/style.css");

    // Drive letters like z: should not be parsed as scheme "z"
    let joined_drive = base.join("z:/foo/bar.css").unwrap();
    // It is treated as path-relative: /app/z:/foo/bar.css
    assert_eq!(joined_drive.scheme(), "https");
    assert_eq!(joined_drive.host(), Some("example.com"));
}

