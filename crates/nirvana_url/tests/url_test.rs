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
