use nirvana_js::JsRuntime;

#[test]
fn test_set_timeout_and_step_timers() {
    let mut runtime = JsRuntime::new();

    runtime
        .eval(
            r#"
            var fired = false;
            var timerId = setTimeout(function() {
                fired = true;
            }, 100);
        "#,
        )
        .expect("eval failed");

    let timer_id = runtime.eval("timerId").expect("eval failed");
    assert_ne!(timer_id, "undefined");
    assert_ne!(timer_id, "0");

    // Advance 50ms (timer should not fire yet)
    let fired_count = runtime.step_timers(50);
    assert_eq!(fired_count, 0);
    assert_eq!(runtime.eval("fired").unwrap(), "false");

    // Advance another 60ms (total 110ms >= 100ms, timer should fire!)
    let fired_count2 = runtime.step_timers(60);
    assert_eq!(fired_count2, 1);
    assert_eq!(runtime.eval("fired").unwrap(), "true");

    // Advance more time (one-shot timer should not fire again)
    let fired_count3 = runtime.step_timers(100);
    assert_eq!(fired_count3, 0);
}

#[test]
fn test_clear_timeout_cancels_timer() {
    let mut runtime = JsRuntime::new();

    runtime
        .eval(
            r#"
            var fired = false;
            var id = setTimeout(function() {
                fired = true;
            }, 50);
            clearTimeout(id);
        "#,
        )
        .expect("eval failed");

    // Advance 100ms
    let count = runtime.step_timers(100);
    assert_eq!(count, 0);
    assert_eq!(runtime.eval("fired").unwrap(), "false");
}
