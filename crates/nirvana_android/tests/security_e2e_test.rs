use nirvana_engine::engine::NirvanaEngine;
use nirvana_net::cookie::Cookie;

#[test]
fn test_e2e_login_flow_with_cookies_and_local_storage() {
    let mut engine = NirvanaEngine::new(400, 400);

    let html = r#"
        <html>
            <head><title>App Login</title></head>
            <body>
                <h1 id="title">Welcome</h1>
                <button id="login-btn">Log In</button>
                <div id="status">Not Logged In</div>
                <script>
                    document.getElementById('login-btn').addEventListener('click', function() {
                        localStorage.setItem('auth_user', 'user_alice');
                        document.cookie = 'user_pref=theme_dark; path=/';
                        document.getElementById('status').textContent = 'Logged In: ' + localStorage.getItem('auth_user');
                    });
                </script>
            </body>
        </html>
    "#;

    engine.load_html(html, None);
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("https://app.example.com/login");
    }
    engine.render();

    assert_eq!(
        engine.evaluate_js("document.getElementById('status').textContent").as_deref(),
        Ok("Not Logged In")
    );

    // Simulate clicking the login button
    // Find button node id
    let dom = engine.dom().expect("DOM exists");
    let btn_id = dom
        .nodes()
        .find(|&n| dom.get_attribute(n, "id") == Some("login-btn"))
        .expect("login-btn found");

    if let Some(js) = engine.js_runtime_mut() {
        let res = js.dispatch_event(btn_id, "click");
        assert_eq!(res, Ok(true));
    }

    // Re-render engine with mutations
    engine.render();

    assert_eq!(
        engine.evaluate_js("document.getElementById('status').textContent").as_deref(),
        Ok("Logged In: user_alice")
    );
    assert_eq!(
        engine.evaluate_js("localStorage.getItem('auth_user')").as_deref(),
        Ok("user_alice")
    );
    assert!(
        engine.evaluate_js("document.cookie").unwrap_or_default().contains("user_pref=theme_dark")
    );
    assert_eq!(engine.pixels().len(), 400 * 400 * 4);
}

#[test]
fn test_e2e_origin_isolation_between_two_sites() {
    let mut engine = NirvanaEngine::new(400, 300);

    // 1. Load Bank site
    engine.load_html("<html><body><h1>Bank</h1></body></html>", None);
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("https://bank.example.com/home");
    }
    let _ = engine.evaluate_js("localStorage.setItem('secret', 'bank_secret_42');");
    let _ = engine.evaluate_js("document.cookie = 'bank_pref=high_security; path=/';");

    assert_eq!(
        engine.evaluate_js("localStorage.getItem('secret')").as_deref(),
        Ok("bank_secret_42")
    );
    assert_eq!(
        engine.evaluate_js("document.cookie").as_deref(),
        Ok("bank_pref=high_security")
    );

    // 2. Navigate / Load Attacker site
    engine.load_html("<html><body><h1>Attacker</h1></body></html>", None);
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("https://attacker.example.com/exploit");
    }

    assert_eq!(
        engine.evaluate_js("localStorage.getItem('secret')").as_deref(),
        Ok("null")
    );
    assert_eq!(
        engine.evaluate_js("document.cookie").as_deref(),
        Ok("")
    );

    // 3. Return to Bank site
    engine.load_html("<html><body><h1>Bank Dashboard</h1></body></html>", None);
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("https://bank.example.com/dashboard");
    }

    assert_eq!(
        engine.evaluate_js("localStorage.getItem('secret')").as_deref(),
        Ok("bank_secret_42")
    );
    assert_eq!(
        engine.evaluate_js("document.cookie").as_deref(),
        Ok("bank_pref=high_security")
    );
}

#[test]
fn test_e2e_httponly_isolation_under_script_execution() {
    let mut engine = NirvanaEngine::new(400, 300);

    engine.load_html("<html><body><h1>Vault</h1></body></html>", None);
    if let Some(js) = engine.js_runtime_mut() {
        js.set_url("https://vault.example.com/account");
    }

    // Server-side sets HttpOnly cookie directly into jar
    {
        let jar = engine.cookie_jar();
        let mut jar_mut = jar.borrow_mut();
        jar_mut.add_cookie(Cookie {
            name: "session_token".to_string(),
            value: "vault_secret_999".to_string(),
            domain: "vault.example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
            same_site: nirvana_net::cookie::SameSite::Strict,
            max_age: None,
            host_only: false,
        });
        jar_mut.add_cookie(Cookie {
            name: "display_theme".to_string(),
            value: "contrast".to_string(),
            domain: "vault.example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: false,
            same_site: nirvana_net::cookie::SameSite::Lax,
            max_age: None,
            host_only: false,
        });
    }

    // Script reads document.cookie: must only see display_theme
    let visible_cookie = engine.evaluate_js("document.cookie").unwrap_or_default();
    assert_eq!(visible_cookie, "display_theme=contrast");
    assert!(!visible_cookie.contains("vault_secret_999"));

    // Malicious script attempts to overwrite HttpOnly cookie from client
    let _ = engine.evaluate_js("document.cookie = 'session_token=hacked; HttpOnly; path=/';");

    // Underlying CookieJar check: RFC 6265 §5.3 Step 11 requires that non-HTTP APIs
    // cannot overwrite an existing HttpOnly cookie!
    let jar = engine.cookie_jar();
    let jar_ref = jar.borrow();
    let cookies = jar_ref.get_cookies_for_url_str("https://vault.example.com/account", true);
    let session_cookie = cookies.iter().find(|c| c.name == "session_token").expect("cookie exists");
    // Crucial security invariant: existing HttpOnly cookie is preserved, not overwritten or degraded
    assert_eq!(session_cookie.value, "vault_secret_999");
    assert!(session_cookie.http_only);
}
