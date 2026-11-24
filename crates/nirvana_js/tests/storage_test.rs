use nirvana_js::JsRuntime;

#[test]
fn test_local_storage_crud_and_length() {
    let mut runtime = JsRuntime::new();
    runtime.set_origin("https://example.com");

    // Initially empty
    let len = runtime.eval("localStorage.length").expect("eval length");
    assert_eq!(len, "0");

    // setItem
    runtime
        .eval("localStorage.setItem('user', 'aryan');")
        .expect("setItem failed");
    runtime
        .eval("localStorage.setItem('role', 'admin');")
        .expect("setItem failed");

    // length
    let len2 = runtime.eval("localStorage.length").expect("eval length");
    assert_eq!(len2, "2");

    // getItem
    let user = runtime.eval("localStorage.getItem('user')").expect("getItem");
    assert_eq!(user, "aryan");

    let role = runtime.eval("localStorage.getItem('role')").expect("getItem");
    assert_eq!(role, "admin");

    let missing = runtime.eval("localStorage.getItem('nonexistent')").expect("getItem");
    assert_eq!(missing, "null");

    // key(index)
    let k0 = runtime.eval("localStorage.key(0)").expect("key(0)");
    assert_eq!(k0, "user");

    // removeItem
    runtime
        .eval("localStorage.removeItem('user');")
        .expect("removeItem failed");
    let user_after = runtime.eval("localStorage.getItem('user')").expect("getItem");
    assert_eq!(user_after, "null");
    let len3 = runtime.eval("localStorage.length").expect("eval length");
    assert_eq!(len3, "1");

    // clear
    runtime.eval("localStorage.clear();").expect("clear failed");
    let len4 = runtime.eval("localStorage.length").expect("eval length");
    assert_eq!(len4, "0");
}

#[test]
fn test_session_storage_independent_of_local_storage() {
    let mut runtime = JsRuntime::new();
    runtime.set_origin("https://example.com");

    runtime
        .eval("localStorage.setItem('theme', 'dark');")
        .expect("set local");
    runtime
        .eval("sessionStorage.setItem('theme', 'light');")
        .expect("set session");

    let local_theme = runtime.eval("localStorage.getItem('theme')").expect("get local");
    let session_theme = runtime.eval("sessionStorage.getItem('theme')").expect("get session");

    assert_eq!(local_theme, "dark");
    assert_eq!(session_theme, "light");
}

#[test]
fn test_storage_origin_isolation() {
    let mut runtime = JsRuntime::new();

    // Site A
    runtime.set_origin("https://site-a.com");
    runtime
        .eval("localStorage.setItem('token', 'secret_a');")
        .expect("set a");

    // Switch origin to Site B
    runtime.set_origin("https://site-b.com");
    let token_b = runtime.eval("localStorage.getItem('token')").expect("get b");
    assert_eq!(token_b, "null");

    // Set token for Site B
    runtime
        .eval("localStorage.setItem('token', 'secret_b');")
        .expect("set b");

    // Switch back to Site A
    runtime.set_origin("https://site-a.com");
    let token_a = runtime.eval("localStorage.getItem('token')").expect("get a");
    assert_eq!(token_a, "secret_a");
}
