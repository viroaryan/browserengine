use nirvana_html::tokenizer::Tokenizer;
use nirvana_html::tree_builder::TreeBuilder;
use nirvana_dom::node::NodeType;

#[test]
fn test_tree_construction_nested_document() {
    let html = "<html><head><title>Nirvana</title></head><body><h1>Title</h1><p>Paragraph</p></body></html>";
    let tokens = Tokenizer::new(html).tokenize_all();
    let tree = TreeBuilder::new(tokens).build();

    let root = tree.root_id();
    let children = tree.children(root);
    assert_eq!(children.len(), 1); // <html>
    let html_id = children[0];
    assert_eq!(tree.get_node(html_id).unwrap().node_type(), NodeType::Element);

    let html_children = tree.children(html_id); // <head>, <body>
    assert_eq!(html_children.len(), 2);
}

#[test]
fn test_tree_void_elements_do_not_capture_children() {
    let html = "<div><img src=\"test.png\"><p>After Image</p></div>";
    let tokens = Tokenizer::new(html).tokenize_all();
    let tree = TreeBuilder::new(tokens).build();

    let root = tree.root_id();
    let html_node = tree.children(root)[0];
    let body_node = tree.children(html_node).iter().find(|&&id| {
        if let nirvana_dom::node::NodeData::Element(data) = &tree.get_node(id).unwrap().data {
            data.tag_name == "body"
        } else {
            false
        }
    }).copied().unwrap_or(html_node);

    let div_id = tree.children(body_node)[0];
    let div_children = tree.children(div_id);
    assert_eq!(div_children.len(), 2); // <img> and <p> are siblings under <div>
}

#[test]
fn test_standard_html_with_newlines_and_head_body_siblings() {
    let html = "<!DOCTYPE html>\n<html>\n  <head><title>Test</title></head>\n  <body class=\"main\"><p>Hi</p></body>\n</html>\n";
    let tokens = Tokenizer::new(html).tokenize_all();
    let tree = TreeBuilder::new(tokens).build();

    let root = tree.root_id();
    let root_children = tree.children(root);
    assert_eq!(root_children.len(), 1); // Only <html> under Document root

    let html_id = root_children[0];
    let html_children = tree.children(html_id);
    assert_eq!(html_children.len(), 2); // <head> and <body> as siblings

    let head_id = html_children[0];
    let body_id = html_children[1];

    if let nirvana_dom::node::NodeData::Element(data) = &tree.get_node(head_id).unwrap().data {
        assert_eq!(data.tag_name, "head");
    } else {
        panic!("First child must be head");
    }

    if let nirvana_dom::node::NodeData::Element(data) = &tree.get_node(body_id).unwrap().data {
        assert_eq!(data.tag_name, "body");
        assert_eq!(tree.get_attribute(body_id, "class"), Some("main"));
    } else {
        panic!("Second child must be body");
    }
}
