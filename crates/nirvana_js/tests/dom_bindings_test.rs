use std::cell::RefCell;
use std::rc::Rc;
use nirvana_dom::NodeType;
use nirvana_html::parse_html;
use nirvana_js::JsRuntime;

#[test]
fn test_dom_get_element_by_id_and_text_content() {
    let html = r#"
        <html>
            <body>
                <div id="greeting" class="msg">Hello <b>World</b></div>
            </body>
        </html>
    "#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    let res = runtime
        .eval("document.getElementById('greeting').textContent")
        .expect("eval failed");
    assert_eq!(res, "Hello World");

    // Mutate textContent
    runtime
        .eval("document.getElementById('greeting').textContent = 'Greetings!';")
        .expect("eval failed");

    assert!(runtime.has_mutated());

    let res2 = runtime
        .eval("document.getElementById('greeting').textContent")
        .expect("eval failed");
    assert_eq!(res2, "Greetings!");

    // Verify in underlying DomTree
    let dom_ref = dom.borrow();
    let mut greeting_id = None;
    for node in dom_ref.nodes() {
        if dom_ref.get_attribute(node, "id") == Some("greeting") {
            greeting_id = Some(node);
            break;
        }
    }
    let greeting_id = greeting_id.expect("greeting element found");
    let children = dom_ref.children(greeting_id);
    assert_eq!(children.len(), 1);
    assert_eq!(dom_ref.node_type(children[0]), Some(NodeType::Text));
}

#[test]
fn test_dom_get_set_attribute() {
    let html = r#"<div id="target" class="initial" data-count="5"></div>"#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    let cls = runtime
        .eval("document.getElementById('target').getAttribute('class')")
        .expect("eval failed");
    assert_eq!(cls, "initial");

    runtime
        .eval("document.getElementById('target').setAttribute('class', 'updated');")
        .expect("eval failed");
    runtime
        .eval("document.getElementById('target').setAttribute('title', 'hover text');")
        .expect("eval failed");

    assert!(runtime.has_mutated());

    let updated_cls = runtime
        .eval("document.getElementById('target').getAttribute('class')")
        .expect("eval failed");
    assert_eq!(updated_cls, "updated");

    let title = runtime
        .eval("document.getElementById('target').getAttribute('title')")
        .expect("eval failed");
    assert_eq!(title, "hover text");
}

#[test]
fn test_dom_inner_html_mutation() {
    let html = r#"<div id="container">Old Content</div>"#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    runtime
        .eval("document.getElementById('container').innerHTML = '<p>New <b>Bold</b> Text</p>';")
        .expect("eval failed");

    assert!(runtime.has_mutated());

    let res = runtime
        .eval("document.getElementById('container').textContent")
        .expect("eval failed");
    assert_eq!(res, "New Bold Text");
}

#[test]
fn test_dom_style_mutation() {
    let html = r#"<div id="box" style="color: red; width: 100px;"></div>"#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    runtime
        .eval("document.getElementById('box').style.color = 'blue';")
        .expect("eval failed");

    let style = runtime
        .eval("document.getElementById('box').getAttribute('style')")
        .expect("eval failed");
    assert!(style.contains("color: blue") || style.contains("color:blue"));
}

#[test]
fn test_dom_input_value_property() {
    let html = r#"<input id="username" value="admin">"#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    let val = runtime
        .eval("document.getElementById('username').value")
        .expect("eval failed");
    assert_eq!(val, "admin");

    runtime
        .eval("document.getElementById('username').value = 'root';")
        .expect("eval failed");

    let val2 = runtime
        .eval("document.getElementById('username').value")
        .expect("eval failed");
    assert_eq!(val2, "root");

    let dom_ref = dom.borrow();
    for node in dom_ref.nodes() {
        if dom_ref.get_attribute(node, "id") == Some("username") {
            assert_eq!(dom_ref.get_attribute(node, "value"), Some("root"));
        }
    }
}

#[test]
fn test_inner_html_entity_escaping_and_void_elements() {
    let html = r#"<div id="wrapper"><input id="inp" type="text"><p title="&quot;quotes&quot;">a &lt; b &amp; c</p></div>"#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    let inner = runtime
        .eval("document.getElementById('wrapper').innerHTML")
        .expect("eval failed");

    // Void element <input> must not have closing </input>
    assert!(!inner.contains("</input>"));
    assert!(inner.contains("<input"));

    // Entities must be escaped
    assert!(inner.contains("a &lt; b &amp; c"));
    assert!(inner.contains("&quot;quotes&quot;"));

    // Round-trip innerHTML and check textContent
    runtime
        .eval("document.getElementById('wrapper').innerHTML = document.getElementById('wrapper').innerHTML;")
        .expect("roundtrip eval failed");

    let text = runtime
        .eval("document.getElementById('wrapper').textContent")
        .expect("eval failed");
    assert_eq!(text, "a < b & c");
}

#[test]
fn test_style_property_removal_on_empty_string() {
    let html = r#"<div id="card" style="color: red; width: 100px;"></div>"#;
    let dom = Rc::new(RefCell::new(parse_html(html)));
    let mut runtime = JsRuntime::with_dom(dom.clone());

    // Setting style property to empty string removes it
    runtime
        .eval("document.getElementById('card').style.color = '';")
        .expect("eval failed");

    let style = runtime
        .eval("document.getElementById('card').getAttribute('style')")
        .expect("eval failed");
    assert!(!style.contains("color"));
    assert!(style.contains("width: 100px"));

    // Removing remaining property cleans up style attribute entirely
    runtime
        .eval("document.getElementById('card').style.width = '';")
        .expect("eval failed");

    let style2 = runtime
        .eval("document.getElementById('card').getAttribute('style')")
        .expect("eval failed");
    assert_eq!(style2, "null");
}
