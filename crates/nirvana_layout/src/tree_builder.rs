//! Layout tree construction from styled DOM nodes.

use crate::box_type::{BoxType, LayoutBox};
use nirvana_core::arena::Id;
use nirvana_dom::node::{Node, NodeData};
use nirvana_dom::{DomTree, NodeType};
use nirvana_style::compute::StyleTree;
use nirvana_style::values::Display;

/// Builds the root of the layout tree from the given DOM tree and computed style tree.
/// Elements with `display: none` (like `<head>`, `<script>`, `<style>`) are omitted.
pub fn build_layout_tree(dom: &DomTree, style_tree: &StyleTree) -> Option<LayoutBox> {
    let root = dom.root_id();
    let root_element = dom
        .children(root)
        .into_iter()
        .find(|&c| dom.node_type(c) == Some(NodeType::Element))?;

    build_box(root_element, dom, style_tree)
}

fn build_box(node_id: Id<Node>, dom: &DomTree, style_tree: &StyleTree) -> Option<LayoutBox> {
    let node_type = dom.node_type(node_id)?;

    let box_type = match node_type {
        NodeType::Element => {
            let style = style_tree.get_style(node_id)?;
            match style.display {
                Display::None => return None,
                Display::Block => BoxType::BlockNode(node_id),
                Display::Inline => BoxType::InlineNode(node_id),
            }
        }
        NodeType::Text => {
            // Check if text is non-empty / meaningful
            if let Some(node) = dom.get_node(node_id)
                && let NodeData::Text(ref text) = node.data
                && text.trim().is_empty()
            {
                return None;
            }
            BoxType::InlineNode(node_id)
        }
        NodeType::Comment | NodeType::Document => return None,
    };

    let mut raw_children = Vec::new();
    for child in dom.children(node_id) {
        if let Some(child_box) = build_box(child, dom, style_tree) {
            raw_children.push(child_box);
        }
    }

    let is_block_container = matches!(box_type, BoxType::BlockNode(_));
    let has_block_child = raw_children.iter().any(|c| c.is_block_level());
    let has_inline_child = raw_children.iter().any(|c| c.is_inline_level());

    let final_children = if is_block_container && has_block_child && has_inline_child {
        // Coalesce inline children into AnonymousBlock boxes per CSS 2.1 §9.2.1.1
        let mut coalesced = Vec::new();
        let mut inline_run = Vec::new();

        for child in raw_children {
            if child.is_inline_level() {
                inline_run.push(child);
            } else {
                if !inline_run.is_empty() {
                    let mut anon = LayoutBox::new(BoxType::AnonymousBlock);
                    anon.children = std::mem::take(&mut inline_run);
                    coalesced.push(anon);
                }
                coalesced.push(child);
            }
        }

        if !inline_run.is_empty() {
            let mut anon = LayoutBox::new(BoxType::AnonymousBlock);
            anon.children = inline_run;
            coalesced.push(anon);
        }

        coalesced
    } else {
        raw_children
    };

    let mut layout_box = LayoutBox::new(box_type);
    layout_box.children = final_children;
    Some(layout_box)
}
