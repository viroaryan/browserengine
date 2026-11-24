use nirvana_js::{JsRuntime, LogLevel};

#[test]
fn test_js_runtime_eval_basic_arithmetic() {
    let mut runtime = JsRuntime::new();
    let result = runtime.eval("1 + 2").expect("eval failed");
    assert_eq!(result, "3");
}

#[test]
fn test_js_runtime_eval_string_manipulation() {
    let mut runtime = JsRuntime::new();
    let result = runtime.eval("'Hello ' + 'Nirvana'").expect("eval failed");
    assert_eq!(result, "Hello Nirvana");
}

#[test]
fn test_js_runtime_eval_syntax_error() {
    let mut runtime = JsRuntime::new();
    let result = runtime.eval("function { invalid syntax");
    assert!(result.is_err());
}

#[test]
fn test_js_runtime_console_log_capture() {
    let mut runtime = JsRuntime::new();
    runtime
        .eval("console.log('User logged in:', 42); console.error('An error occurred!');")
        .expect("eval failed");

    let logs = runtime.logs();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].level, LogLevel::Log);
    assert_eq!(logs[0].message, "User logged in: 42");
    assert_eq!(logs[1].level, LogLevel::Error);
    assert_eq!(logs[1].message, "An error occurred!");
}
