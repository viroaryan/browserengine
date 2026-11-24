use std::cell::RefCell;
use std::rc::Rc;
use nirvana_html::parse_html;
use nirvana_js::JsRuntime;

#[test]
fn test_add_event_listener_and_dispatch_click() {
    let html = r#"
        <div id="container">
            <button id="btn">Click Me</button>
        </div>
    "#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    runtime
        .eval(
            r#"
            var clicked = false;
            var btn = document.getElementById('btn');
            btn.addEventListener('click', function(e) {
                clicked = true;
                btn.textContent = 'Clicked!';
            });
        "#,
        )
        .expect("eval failed");

    // Discover button node id
    let dom_ref = dom.borrow();
    let btn_id = dom_ref
        .nodes()
        .find(|&n| dom_ref.get_attribute(n, "id") == Some("btn"))
        .expect("button found");
    drop(dom_ref);

    // Dispatch event from native engine
    runtime
        .dispatch_event(btn_id, "click")
        .expect("dispatch failed");

    let clicked_val = runtime.eval("clicked").expect("eval failed");
    assert_eq!(clicked_val, "true");

    let text_val = runtime
        .eval("document.getElementById('btn').textContent")
        .expect("eval failed");
    assert_eq!(text_val, "Clicked!");
}

#[test]
fn test_event_bubbling_to_ancestor() {
    let html = r#"
        <div id="parent">
            <span id="child">Target</span>
        </div>
    "#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    runtime
        .eval(
            r#"
            var parentFired = false;
            var childFired = false;
            document.getElementById('parent').addEventListener('click', function(e) {
                parentFired = true;
            });
            document.getElementById('child').addEventListener('click', function(e) {
                childFired = true;
            });
        "#,
        )
        .expect("eval failed");

    let dom_ref = dom.borrow();
    let child_id = dom_ref
        .nodes()
        .find(|&n| dom_ref.get_attribute(n, "id") == Some("child"))
        .expect("child found");
    drop(dom_ref);

    runtime
        .dispatch_event(child_id, "click")
        .expect("dispatch failed");

    assert_eq!(runtime.eval("childFired").unwrap(), "true");
    assert_eq!(runtime.eval("parentFired").unwrap(), "true");
}

#[test]
fn test_event_stop_propagation() {
    let html = r#"
        <div id="parent">
            <span id="child">Target</span>
        </div>
    "#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    runtime
        .eval(
            r#"
            var parentFired = false;
            document.getElementById('parent').addEventListener('click', function(e) {
                parentFired = true;
            });
            document.getElementById('child').addEventListener('click', function(e) {
                e.stopPropagation();
            });
        "#,
        )
        .expect("eval failed");

    let dom_ref = dom.borrow();
    let child_id = dom_ref
        .nodes()
        .find(|&n| dom_ref.get_attribute(n, "id") == Some("child"))
        .expect("child found");
    drop(dom_ref);

    runtime
        .dispatch_event(child_id, "click")
        .expect("dispatch failed");

    assert_eq!(runtime.eval("parentFired").unwrap(), "false");
}
