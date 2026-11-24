use nirvana_css::parser::parse_css;
use nirvana_dom::DomTree;
use nirvana_style::matcher::matches_selector;

#[test]
fn test_selector_matching_tag_class_id() {
    let mut tree = DomTree::new();
    let root = tree.root_id();
    let btn = tree.create_element("button");
    tree.set_attribute(btn, "class", "btn primary").unwrap();
    tree.set_attribute(btn, "id", "submit-btn").unwrap();
    tree.append_child(root, btn).unwrap();

    let css = "button { } .btn { } .primary { } #submit-btn { } .btn.primary { } div { } .secondary { }";
    let stylesheet = parse_css(css);

    // button matches
    assert!(matches_selector(btn, &stylesheet.rules[0].selectors[0], &tree));
    // .btn matches
    assert!(matches_selector(btn, &stylesheet.rules[1].selectors[0], &tree));
    // .primary matches
    assert!(matches_selector(btn, &stylesheet.rules[2].selectors[0], &tree));
    // #submit-btn matches
    assert!(matches_selector(btn, &stylesheet.rules[3].selectors[0], &tree));
    // .btn.primary matches compound
    assert!(matches_selector(btn, &stylesheet.rules[4].selectors[0], &tree));
    // div does not match button
    assert!(!matches_selector(btn, &stylesheet.rules[5].selectors[0], &tree));
    // .secondary does not match
    assert!(!matches_selector(btn, &stylesheet.rules[6].selectors[0], &tree));
}

#[test]
fn test_universal_selector_matching() {
    use nirvana_css::ast::{Selector, SimpleSelector};

    let mut tree = DomTree::new();
    let root = tree.root_id();
    let div = tree.create_element("div");
    tree.append_child(root, div).unwrap();
    let text = tree.create_text("hello");
    tree.append_child(div, text).unwrap();

    let universal = Selector::new(vec![SimpleSelector::Universal]);

    // Matches element
    assert!(matches_selector(div, &universal, &tree));
    // Does not match text node
    assert!(!matches_selector(text, &universal, &tree));
}
