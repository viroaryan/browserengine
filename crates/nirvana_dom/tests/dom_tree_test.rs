use nirvana_dom::tree::{DomTree, DomError};
use nirvana_dom::node::NodeType;

#[test]
fn test_dom_tree_construction_and_traversal() {
    let mut tree = DomTree::new();
    let root = tree.root_id();
    
    let html = tree.create_element("html");
    let body = tree.create_element("body");
    let p = tree.create_element("p");
    let text = tree.create_text("Hello Nirvana Engine");

    tree.append_child(root, html).unwrap();
    tree.append_child(html, body).unwrap();
    tree.append_child(body, p).unwrap();
    tree.append_child(p, text).unwrap();

    assert_eq!(tree.parent(text), Some(p));
    assert_eq!(tree.parent(p), Some(body));
    assert_eq!(tree.children(body), vec![p]);
    assert_eq!(tree.children(p), vec![text]);
    assert_eq!(tree.node_type(p), NodeType::Element);
    assert_eq!(tree.node_type(text), NodeType::Text);
}

#[test]
fn test_dom_cycle_prevention() {
    let mut tree = DomTree::new();
    let div1 = tree.create_element("div");
    let div2 = tree.create_element("div");

    tree.append_child(div1, div2).unwrap();
    // Attempting to append parent as a child of its own child must fail
    assert!(matches!(tree.append_child(div2, div1), Err(DomError::CycleDetected)));
}

#[test]
fn test_dom_element_attributes() {
    let mut tree = DomTree::new();
    let link = tree.create_element("a");
    tree.set_attribute(link, "href", "https://nirvanabrowser.org").unwrap();
    tree.set_attribute(link, "target", "_blank").unwrap();

    assert_eq!(tree.get_attribute(link, "href"), Some("https://nirvanabrowser.org"));
    assert_eq!(tree.get_attribute(link, "target"), Some("_blank"));
    assert_eq!(tree.get_attribute(link, "id"), None);
}
