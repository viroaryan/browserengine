//! CSS selector matching engine for DOM elements.

use nirvana_core::arena::Id;
use nirvana_css::ast::{AttrOp, Combinator, Selector, SimpleSelector};
use nirvana_dom::node::Node;
use nirvana_dom::{DomTree, NodeType};

/// Checks whether a given DOM node matches a CSS selector (including ancestor/sibling combinators).
/// Only `NodeType::Element` nodes can match CSS selectors.
pub fn matches_selector(node_id: Id<Node>, selector: &Selector, dom: &DomTree) -> bool {
    if dom.node_type(node_id) != Some(NodeType::Element) {
        return false;
    }

    if !matches_compound(node_id, &selector.simple_selectors, dom) {
        return false;
    }

    if selector.ancestors.is_empty() {
        return true;
    }

    match_combinator_chain(node_id, &selector.ancestors, 0, dom)
}

fn match_combinator_chain(
    current_id: Id<Node>,
    segments: &[nirvana_css::ast::SelectorSegment],
    idx: usize,
    dom: &DomTree,
) -> bool {
    if idx >= segments.len() {
        return true;
    }
    let seg = &segments[idx];

    match seg.combinator {
        Combinator::Child => {
            if let Some(parent_id) = dom.parent(current_id)
                && dom.node_type(parent_id) == Some(NodeType::Element)
                && matches_compound(parent_id, &seg.simple_selectors, dom)
            {
                return match_combinator_chain(parent_id, segments, idx + 1, dom);
            }
            false
        }
        Combinator::Descendant => {
            let mut cursor = dom.parent(current_id);
            while let Some(anc_id) = cursor {
                if dom.node_type(anc_id) == Some(NodeType::Element)
                    && matches_compound(anc_id, &seg.simple_selectors, dom)
                    && match_combinator_chain(anc_id, segments, idx + 1, dom)
                {
                    return true;
                }
                cursor = dom.parent(anc_id);
            }
            false
        }
        Combinator::NextSibling => {
            if let Some(prev_elem) = prev_element_sibling(dom, current_id)
                && matches_compound(prev_elem, &seg.simple_selectors, dom)
            {
                return match_combinator_chain(prev_elem, segments, idx + 1, dom);
            }
            false
        }
        Combinator::SubsequentSibling => {
            let mut cursor = prev_element_sibling(dom, current_id);
            while let Some(sib_id) = cursor {
                if matches_compound(sib_id, &seg.simple_selectors, dom)
                    && match_combinator_chain(sib_id, segments, idx + 1, dom)
                {
                    return true;
                }
                cursor = prev_element_sibling(dom, sib_id);
            }
            false
        }
    }
}

fn prev_element_sibling(dom: &DomTree, node_id: Id<Node>) -> Option<Id<Node>> {
    let parent = dom.parent(node_id)?;
    let children = dom.children(parent);
    let pos = children.iter().position(|&id| id == node_id)?;
    for &sib in children[..pos].iter().rev() {
        if dom.node_type(sib) == Some(NodeType::Element) {
            return Some(sib);
        }
    }
    None
}

fn next_element_sibling(dom: &DomTree, node_id: Id<Node>) -> Option<Id<Node>> {
    let parent = dom.parent(node_id)?;
    let children = dom.children(parent);
    let pos = children.iter().position(|&id| id == node_id)?;
    for &sib in &children[pos + 1..] {
        if dom.node_type(sib) == Some(NodeType::Element) {
            return Some(sib);
        }
    }
    None
}

fn matches_compound(node_id: Id<Node>, simple_selectors: &[SimpleSelector], dom: &DomTree) -> bool {
    for simple in simple_selectors {
        if !matches_simple_selector(node_id, simple, dom) {
            return false;
        }
    }
    true
}

fn matches_simple_selector(node_id: Id<Node>, simple: &SimpleSelector, dom: &DomTree) -> bool {
    match simple {
        SimpleSelector::Universal => true,
        SimpleSelector::Tag(target_tag) => dom
            .tag_name(node_id)
            .is_some_and(|tag_name| tag_name.eq_ignore_ascii_case(target_tag)),
        SimpleSelector::Id(target_id) => dom
            .get_attribute(node_id, "id")
            .is_some_and(|id_val| id_val == target_id),
        SimpleSelector::Class(target_class) => dom.get_attribute(node_id, "class").is_some_and(|cls| {
            cls.split_ascii_whitespace().any(|c| c == target_class)
        }),
        SimpleSelector::Attribute { name, op, value } => {
            let Some(attr_val) = dom.get_attribute(node_id, name) else {
                return false;
            };
            match op {
                AttrOp::Exists => true,
                AttrOp::Exact => value.as_deref() == Some(attr_val),
                AttrOp::Includes => value
                    .as_deref()
                    .is_some_and(|v| !v.is_empty() && attr_val.split_ascii_whitespace().any(|w| w == v)),
                AttrOp::DashMatch => value.as_deref().is_some_and(|v| {
                    attr_val == v || attr_val.starts_with(&format!("{v}-"))
                }),
                AttrOp::Prefix => value
                    .as_deref()
                    .is_some_and(|v| !v.is_empty() && attr_val.starts_with(v)),
                AttrOp::Suffix => value
                    .as_deref()
                    .is_some_and(|v| !v.is_empty() && attr_val.ends_with(v)),
                AttrOp::Contains => value
                    .as_deref()
                    .is_some_and(|v| !v.is_empty() && attr_val.contains(v)),
            }
        }
        SimpleSelector::PseudoClass(pseudo) => match pseudo.as_str() {
            "link" => {
                dom.tag_name(node_id).is_some_and(|t| t.eq_ignore_ascii_case("a"))
                    && dom.get_attribute(node_id, "href").is_some()
            }
            "first-child" => prev_element_sibling(dom, node_id).is_none(),
            "last-child" => next_element_sibling(dom, node_id).is_none(),
            "only-child" => {
                prev_element_sibling(dom, node_id).is_none()
                    && next_element_sibling(dom, node_id).is_none()
            }
            "root" => dom
                .tag_name(node_id)
                .is_some_and(|t| t.eq_ignore_ascii_case("html")),
            // Dynamic / pseudo-element selectors do not match default element rest state
            _ => false,
        },
    }
}
