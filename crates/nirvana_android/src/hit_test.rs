//! Hit test result information describing the tapped DOM target.

use nirvana_core::arena::Id;
use nirvana_dom::node::{Node, NodeData};
use nirvana_dom::{DomTree, NodeType};

/// Result of a hit-test operation identifying the topmost interactive or structural target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitTestResult {
    /// Tag name of the element (e.g. "a", "div", "button", "p").
    pub tag: String,
    /// Destination URL if the clicked element or any of its ancestors is an anchor link (`<a>`).
    pub href: Option<String>,
    /// Text content inside the hit element, if any.
    pub text: Option<String>,
}

/// Resolves a DOM node ID returned by the layout hit-test into a high-level `HitTestResult`.
/// Walks ancestor elements to discover enclosing `<a>` tags with `href` attributes.
pub fn resolve_hit_target(dom: &DomTree, hit_node_id: Id<Node>) -> HitTestResult {
    let mut current = Some(hit_node_id);
    let mut primary_element_id = None;
    let mut anchor_node_id = None;

    // Traverse ancestors looking for the containing element and any enclosing <a> anchor
    while let Some(curr_id) = current {
        if dom.node_type(curr_id) == Some(NodeType::Element) {
            if primary_element_id.is_none() {
                primary_element_id = Some(curr_id);
            }
            if dom
                .tag_name(curr_id)
                .map(|t| t.eq_ignore_ascii_case("a"))
                .unwrap_or(false)
            {
                anchor_node_id = Some(curr_id);
                break;
            }
        }
        current = dom.parent(curr_id);
    }

    if let Some(anchor_id) = anchor_node_id {
        let href = dom.get_attribute(anchor_id, "href").map(String::from);
        let text_content = collect_element_text(dom, anchor_id);
        let text = if text_content.is_empty() {
            None
        } else {
            Some(text_content)
        };
        HitTestResult {
            tag: "a".to_string(),
            href,
            text,
        }
    } else if let Some(elem_id) = primary_element_id {
        let tag = dom.tag_name(elem_id).unwrap_or("div").to_string();
        let href = dom.get_attribute(elem_id, "href").map(String::from);
        let text_content = collect_element_text(dom, elem_id);
        let text = if text_content.is_empty() {
            None
        } else {
            Some(text_content)
        };
        HitTestResult { tag, href, text }
    } else {
        HitTestResult {
            tag: "div".to_string(),
            href: None,
            text: None,
        }
    }
}

fn collect_element_text(dom: &DomTree, node_id: Id<Node>) -> String {
    let mut text = String::new();
    if let Some(node) = dom.get_node(node_id) {
        match &node.data {
            NodeData::Text(t) => text.push_str(t.trim()),
            NodeData::Element(_) => {
                for child in dom.children(node_id) {
                    let child_text = collect_element_text(dom, child);
                    if !child_text.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&child_text);
                    }
                }
            }
            _ => {}
        }
    }
    text
}
