use std::cell::RefCell;
use std::rc::Rc;

use nirvana_html::parse_html;
use nirvana_js::runtime::JsRuntime;
use nirvana_net::cookie::Cookie;

#[test]
fn test_document_cookie_read_filters_http_only() {
    let mut runtime = JsRuntime::new();
    let dom = Rc::new(RefCell::new(parse_html("<html><body></body></html>")));
    runtime.bind_dom(dom).unwrap();
    runtime.set_url("https://example.com/app");

    // Pre-populate cookie jar with one HttpOnly and one public cookie
    {
        let jar = runtime.cookie_jar();
        let mut jar_mut = jar.borrow_mut();
        jar_mut.add_cookie(Cookie {
            name: "auth_token".to_string(),
            value: "secret_val".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
            same_site: nirvana_net::cookie::SameSite::Lax,
            max_age: None,
        });
        jar_mut.add_cookie(Cookie {
            name: "theme".to_string(),
            value: "dark".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: false,
            same_site: nirvana_net::cookie::SameSite::Lax,
            max_age: None,
        });
    }

    let cookie_val = runtime.eval("document.cookie").unwrap();
    assert_eq!(cookie_val, "theme=dark");
    assert!(!cookie_val.contains("auth_token"));
}

#[test]
fn test_document_cookie_write_updates_jar() {
    let mut runtime = JsRuntime::new();
    let dom = Rc::new(RefCell::new(parse_html("<html><body></body></html>")));
    runtime.bind_dom(dom).unwrap();
    runtime.set_url("https://example.com/profile");

    runtime.eval("document.cookie = 'user_id=42; path=/';").unwrap();
    let cookie_val = runtime.eval("document.cookie").unwrap();
    assert_eq!(cookie_val, "user_id=42");

    // Verify in underlying CookieJar
    let jar = runtime.cookie_jar();
    let jar_ref = jar.borrow();
    let cookies = jar_ref.get_cookies_for_url_str("https://example.com/profile", false);
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "user_id");
    assert_eq!(cookies[0].value, "42");
    assert!(!cookies[0].http_only);
}

#[test]
fn test_document_cookie_cannot_set_http_only_from_js() {
    let mut runtime = JsRuntime::new();
    let dom = Rc::new(RefCell::new(parse_html("<html><body></body></html>")));
    runtime.bind_dom(dom).unwrap();
    runtime.set_url("https://example.com/");

    // Attempt to set HttpOnly via client-side script
    runtime.eval("document.cookie = 'exploit=bad; HttpOnly';").unwrap();

    let jar = runtime.cookie_jar();
    let jar_ref = jar.borrow();
    let cookies = jar_ref.get_cookies_for_url_str("https://example.com/", false);
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "exploit");
    // Client-side script must NOT be able to flag cookies as http_only
    assert!(!cookies[0].http_only);
}

#[test]
fn test_document_cookie_multiple_cookies() {
    let mut runtime = JsRuntime::new();
    let dom = Rc::new(RefCell::new(parse_html("<html><body></body></html>")));
    runtime.bind_dom(dom).unwrap();
    runtime.set_url("https://example.com/");

    runtime.eval("document.cookie = 'c1=v1; path=/';").unwrap();
    runtime.eval("document.cookie = 'c2=v2; path=/';").unwrap();

    let cookie_val = runtime.eval("document.cookie").unwrap();
    assert!(cookie_val.contains("c1=v1"));
    assert!(cookie_val.contains("c2=v2"));
    assert!(cookie_val.contains("; "));
}
