use nirvana_net::cookie::{parse_set_cookie, Cookie, CookieJar, SameSite};
use nirvana_url::Url;

#[test]
fn test_parse_set_cookie_full_attributes() {
    let header = "session_id=xyz123; Domain=example.com; Path=/app; Secure; HttpOnly; SameSite=Strict; Max-Age=3600";
    let cookie = parse_set_cookie(header, "default.com", "/").expect("cookie parsed");

    assert_eq!(cookie.name, "session_id");
    assert_eq!(cookie.value, "xyz123");
    assert_eq!(cookie.domain, "example.com");
    assert_eq!(cookie.path, "/app");
    assert!(cookie.secure);
    assert!(cookie.http_only);
    assert_eq!(cookie.same_site, SameSite::Strict);
    assert_eq!(cookie.max_age, Some(3600));
}

#[test]
fn test_parse_set_cookie_defaults() {
    let header = "theme=dark";
    let cookie = parse_set_cookie(header, "myblog.org", "/posts").expect("cookie parsed");

    assert_eq!(cookie.name, "theme");
    assert_eq!(cookie.value, "dark");
    assert_eq!(cookie.domain, "myblog.org");
    assert_eq!(cookie.path, "/posts");
    assert!(!cookie.secure);
    assert!(!cookie.http_only);
    assert_eq!(cookie.same_site, SameSite::Lax); // Default SameSite
    assert_eq!(cookie.max_age, None);
}

#[test]
fn test_cookie_domain_and_path_matching() {
    let c1 = Cookie {
        name: "c1".to_string(),
        value: "v1".to_string(),
        domain: "example.com".to_string(),
        path: "/api".to_string(),
        secure: false,
        http_only: false,
        same_site: SameSite::Lax,
        max_age: None,
    };

    assert!(c1.domain_matches("example.com"));
    assert!(c1.domain_matches("sub.example.com"));
    assert!(!c1.domain_matches("example.org"));
    assert!(!c1.domain_matches("myexample.com"));

    assert!(c1.path_matches("/api"));
    assert!(c1.path_matches("/api/v1/users"));
    assert!(!c1.path_matches("/apiv1"));
    assert!(!c1.path_matches("/"));
}

#[test]
fn test_cookie_jar_storage_and_header_generation() {
    let mut jar = CookieJar::new();

    let u1 = Url::parse("https://example.com/app/dashboard").unwrap();
    let c_secure = parse_set_cookie(
        "token=secret; Secure; HttpOnly; Path=/app",
        u1.host().unwrap(),
        "/app",
    )
    .unwrap();
    let c_normal = parse_set_cookie(
        "lang=en; Path=/",
        u1.host().unwrap(),
        "/",
    )
    .unwrap();

    jar.store(c_secure, &u1);
    jar.store(c_normal, &u1);

    // 1. HTTPS request with is_http = true (network fetch) -> both token and lang are sent
    let header_https = jar.get_cookie_header_value(&u1, true).expect("header exists");
    assert!(header_https.contains("token=secret"));
    assert!(header_https.contains("lang=en"));

    // 2. HTTP request (insecure) -> secure cookie "token" is excluded
    let u_insecure = Url::parse("http://example.com/app/dashboard").unwrap();
    let header_http = jar.get_cookie_header_value(&u_insecure, true).expect("header exists");
    assert!(!header_http.contains("token=secret"));
    assert!(header_http.contains("lang=en"));

    // 3. Script access on HTTPS (is_http = false, for document.cookie) -> HttpOnly "token" is excluded
    let script_cookies = jar.get_cookies_for_url(&u1, false);
    assert_eq!(script_cookies.len(), 1);
    assert_eq!(script_cookies[0].name, "lang");
    assert_eq!(script_cookies[0].value, "en");
}

#[test]
fn test_cookie_jar_replacement_and_clear() {
    let mut jar = CookieJar::new();
    let url = Url::parse("https://example.com/").unwrap();

    let c_v1 = parse_set_cookie("count=1", "example.com", "/").unwrap();
    jar.store(c_v1, &url);
    assert_eq!(jar.get_cookie_header_value(&url, true).as_deref(), Some("count=1"));

    // Update count to 2
    let c_v2 = parse_set_cookie("count=2", "example.com", "/").unwrap();
    jar.store(c_v2, &url);
    assert_eq!(jar.get_cookie_header_value(&url, true).as_deref(), Some("count=2"));

    // Clear
    jar.clear();
    assert_eq!(jar.get_cookie_header_value(&url, true), None);
}
