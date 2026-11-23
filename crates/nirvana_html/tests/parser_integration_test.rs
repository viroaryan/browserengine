use nirvana_html::parse_html;
use nirvana_dom::node::NodeData;

#[test]
fn test_parse_html_convenience_api() {
    let tree = parse_html("<div class=\"hero\"><h1>Welcome to Nirvana Browser</h1></div>");
    let root = tree.root_id();
    assert!(!tree.children(root).is_empty());
}

#[test]
fn test_parse_html_malformed_unclosed_tags() {
    // Unclosed <p> tags should be handled without panic or orphan nodes
    let tree = parse_html("<div><p>First paragraph<div>Nested block</div>");
    let root = tree.root_id();
    assert!(!tree.children(root).is_empty());
}

#[test]
fn test_parse_html_attribute_retrieval() {
    let tree = parse_html("<a href=\"https://nirvanabrowser.org\" target=\"_blank\">Link</a>");
    let root = tree.root_id();
    
    // Find the <a> tag and verify its attributes
    let mut found = false;
    let mut stack = vec![root];
    while let Some(node_id) = stack.pop() {
        if let Some(node) = tree.get_node(node_id) {
            if let NodeData::Element(data) = &node.data {
                if data.tag_name == "a" {
                    assert_eq!(tree.get_attribute(node_id, "href"), Some("https://nirvanabrowser.org"));
                    assert_eq!(tree.get_attribute(node_id, "target"), Some("_blank"));
                    found = true;
                    break;
                }
            }
            stack.extend(tree.children(node_id));
        }
    }
    assert!(found);
}
