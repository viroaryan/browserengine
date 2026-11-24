use nirvana_engine::engine::NirvanaEngine;
use nirvana_net::cookie::Cookie;
use nirvana_net::security::{is_mixed_content, Origin};
use nirvana_url::Url;

#[test]
fn test_engine_cookie_jar_lifecycle_and_clear() {
    let mut engine = NirvanaEngine::new(800, 600);
    engine.load_html(
        "<html><body><p id='status'>Ready</p></body></html>",
        None,
    );

    // Pre-populate cookie jar
    {
        let jar = engine.cookie_jar();
        let mut jar_mut = jar.borrow_mut();
        jar_mut.add_cookie(Cookie {
            name: "session_id".to_string(),
            value: "xyz123".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: nirvana_net::cookie::SameSite::Lax,
            max_age: None,
        });
        jar_mut.add_cookie(Cookie {
            name: "admin_token".to_string(),
            value: "secret_admin".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: true,
            same_site: nirvana_net::cookie::SameSite::Lax,
            max_age: None,
        });
    }

    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("http://example.com/dashboard");
    }

    let cookie_val = engine.evaluate_js("document.cookie").unwrap_or_default();
    assert_eq!(cookie_val, "session_id=xyz123");
    assert!(!cookie_val.contains("admin_token"));

    // Clear cookies via engine API
    engine.clear_cookies();

    assert_eq!(engine.cookie_jar().borrow().all_cookies().len(), 0);
    let after_clear = engine.evaluate_js("document.cookie").unwrap_or_default();
    assert_eq!(after_clear, "");
}

#[test]
fn test_engine_storage_lifecycle_and_clear() {
    let mut engine = NirvanaEngine::new(800, 600);
    engine.load_html(
        "<html><body><p>Storage Test</p></body></html>",
        None,
    );

    let res = engine.evaluate_js(
        r#"
        localStorage.setItem('user', 'Alice');
        sessionStorage.setItem('temp', '12345');
        localStorage.getItem('user');
        "#,
    );
    assert_eq!(res.as_deref(), Ok("Alice"));

    // Clear storage via engine API
    engine.clear_storage();

    let check_local = engine.evaluate_js("localStorage.getItem('user')");
    assert_eq!(check_local.as_deref(), Ok("null"));
    let check_session = engine.evaluate_js("sessionStorage.getItem('temp')");
    assert_eq!(check_session.as_deref(), Ok("null"));
}

#[test]
fn test_engine_cookie_persistence_across_html_reloads() {
    let mut engine = NirvanaEngine::new(800, 600);

    // Page 1
    engine.load_html(
        "<html><body><h1>Page 1</h1></body></html>",
        None,
    );
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("http://example.com/page1");
    }
    let _ = engine.evaluate_js("document.cookie = 'theme=dark; path=/';");

    // Page 2
    engine.load_html(
        "<html><body><h1>Page 2</h1></body></html>",
        None,
    );
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("http://example.com/page2");
    }
    let cookie_val = engine.evaluate_js("document.cookie").unwrap_or_default();
    assert_eq!(cookie_val, "theme=dark");
}

#[test]
fn test_mixed_content_detection_helper() {
    let https_doc = Url::parse("https://example.com/index.html").unwrap();
    let doc_origin = Origin::from_url(&https_doc);
    let http_script = Url::parse("http://example.com/app.js").unwrap();
    let https_script = Url::parse("https://example.com/app.js").unwrap();

    assert!(is_mixed_content(&doc_origin, &http_script));
    assert!(!is_mixed_content(&doc_origin, &https_script));
}
