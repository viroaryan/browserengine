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
    assert!(!cookie.host_only);
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
    assert!(cookie.host_only);
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
        host_only: false,
    };

    assert!(c1.domain_matches("example.com"));
    assert!(c1.domain_matches("sub.example.com"));
    assert!(!c1.domain_matches("example.org"));
    assert!(!c1.domain_matches("myexample.com"));

    // Host-only cookie must NOT match subdomains
    let c_host_only = Cookie {
        name: "c2".to_string(),
        value: "v2".to_string(),
        domain: "example.com".to_string(),
        path: "/api".to_string(),
        secure: false,
        http_only: false,
        same_site: SameSite::Lax,
        max_age: None,
        host_only: true,
    };
    assert!(c_host_only.domain_matches("example.com"));
    assert!(!c_host_only.domain_matches("sub.example.com"));

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

#[test]
fn test_cross_site_cookie_rejection() {
    let mut jar = CookieJar::new();
    let attacker_url = Url::parse("https://attacker.com/page").unwrap();

    // Attacker tries to set cookie for bank.com
    let stolen = parse_set_cookie("session=evil; Domain=bank.com; Path=/", "attacker.com", "/").unwrap();
    jar.store(stolen, &attacker_url);

    // Cookie must be rejected because attacker.com does not domain-match bank.com
    let bank_url = Url::parse("https://bank.com/").unwrap();
    assert_eq!(jar.get_cookie_header_value(&bank_url, true), None);
}

#[test]
fn test_http_only_cookie_overwrite_prevention() {
    let mut jar = CookieJar::new();
    let url = Url::parse("https://example.com/app").unwrap();

    // 1. Server sets HttpOnly cookie via HTTP
    let server_cookie = parse_set_cookie("auth=secret123; HttpOnly; Path=/", "example.com", "/").unwrap();
    jar.store_with_context(server_cookie, &url, true);

    // 2. Client script attempts to overwrite it via non-HTTP API (document.cookie)
    let client_cookie = parse_set_cookie("auth=hacked; Path=/", "example.com", "/").unwrap();
    jar.store_with_context(client_cookie, &url, false);

    // RFC 6265 §5.3 Step 11: Existing HttpOnly cookie must remain intact!
    let matching = jar.get_cookies_for_url(&url, true);
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].name, "auth");
    assert_eq!(matching[0].value, "secret123");
    assert!(matching[0].http_only);
}

#[test]
fn test_max_age_zero_and_negative_eviction() {
    let mut jar = CookieJar::new();
    let url = Url::parse("https://example.com/").unwrap();

    // Initial cookie
    let c = parse_set_cookie("token=abc; Path=/", "example.com", "/").unwrap();
    jar.store(c, &url);
    assert_eq!(jar.get_cookie_header_value(&url, true).as_deref(), Some("token=abc"));

    // Server sends Max-Age=0 to evict
    let c_evict = parse_set_cookie("token=deleted; Max-Age=0; Path=/", "example.com", "/").unwrap();
    jar.store(c_evict, &url);
    assert_eq!(jar.get_cookie_header_value(&url, true), None);

    // Initial cookie again
    let c2 = parse_set_cookie("token=xyz; Path=/", "example.com", "/").unwrap();
    jar.store(c2, &url);
    assert_eq!(jar.get_cookie_header_value(&url, true).as_deref(), Some("token=xyz"));

    // Server sends negative Max-Age=-1 to evict
    let c_evict_neg = parse_set_cookie("token=deleted; Max-Age=-1; Path=/", "example.com", "/").unwrap();
    jar.store(c_evict_neg, &url);
    assert_eq!(jar.get_cookie_header_value(&url, true), None);
}
