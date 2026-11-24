//! CSS selector matching engine for DOM elements.

use nirvana_core::arena::Id;
use nirvana_css::ast::{Selector, SimpleSelector};
use nirvana_dom::node::Node;
use nirvana_dom::{DomTree, NodeType};

/// Checks whether a given DOM node matches a CSS selector.
/// Only `NodeType::Element` nodes can match CSS selectors.
pub fn matches_selector(node_id: Id<Node>, selector: &Selector, dom: &DomTree) -> bool {
    let node_type = match dom.node_type(node_id) {
        Some(t) => t,
        None => return false,
    };

    if node_type != NodeType::Element {
        return false;
    }

    // A compound selector matches if ALL simple selectors in it match (conjunction)
    for simple in &selector.simple_selectors {
        if !matches_simple_selector(node_id, simple, dom) {
            return false;
        }
    }

    true
}

fn matches_simple_selector(node_id: Id<Node>, simple: &SimpleSelector, dom: &DomTree) -> bool {
    match simple {
        SimpleSelector::Universal => true,
        SimpleSelector::Tag(target_tag) => {
            if let Some(tag_name) = dom.tag_name(node_id) {
                tag_name.eq_ignore_ascii_case(target_tag)
            } else {
                false
            }
        }
        SimpleSelector::Id(target_id) => {
            if let Some(id_val) = dom.get_attribute(node_id, "id") {
                id_val == target_id
            } else {
                false
            }
        }
        SimpleSelector::Class(target_class) => {
            if let Some(class_attr) = dom.get_attribute(node_id, "class") {
                class_attr
                    .split_ascii_whitespace()
                    .any(|c| c == target_class)
            } else {
                false
            }
        }
    }
}
