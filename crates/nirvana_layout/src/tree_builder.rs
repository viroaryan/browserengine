//! Layout tree construction from styled DOM nodes.

use crate::box_type::{BoxType, LayoutBox};
use nirvana_core::arena::Id;
use nirvana_dom::node::{Node, NodeData};
use nirvana_dom::{DomTree, NodeType};
use nirvana_style::compute::StyleTree;
use nirvana_style::values::Display;

use std::collections::HashMap;

/// Context for layout tree construction containing asset information such as image dimensions.
#[derive(Debug, Default, Clone)]
pub struct LayoutContext {
    /// Intrinsic dimensions (width, height) for image element nodes.
    pub image_dimensions: HashMap<Id<Node>, (f32, f32)>,
}

/// Builds the root of the layout tree from the given DOM tree and computed style tree.
/// Elements with `display: none` (like `<head>`, `<script>`, `<style>`) are omitted.
pub fn build_layout_tree(dom: &DomTree, style_tree: &StyleTree) -> Option<LayoutBox> {
    build_layout_tree_with_context(dom, style_tree, &LayoutContext::default())
}

/// Builds the root of the layout tree using additional asset information (such as image dimensions).
pub fn build_layout_tree_with_context(
    dom: &DomTree,
    style_tree: &StyleTree,
    context: &LayoutContext,
) -> Option<LayoutBox> {
    let root = dom.root_id();
    let root_element = dom
        .children(root)
        .into_iter()
        .find(|&c| dom.node_type(c) == Some(NodeType::Element))?;

    build_box(root_element, dom, style_tree, context)
}

fn build_box(
    node_id: Id<Node>,
    dom: &DomTree,
    style_tree: &StyleTree,
    context: &LayoutContext,
) -> Option<LayoutBox> {
    let node_type = dom.node_type(node_id)?;
    let mut initial_size = None;
    let mut intrinsic_size = None;

    let box_type = match node_type {
        NodeType::Element => {
            let style = style_tree.get_style(node_id)?;
            let b_type = match style.display {
                Display::None => return None,
                Display::Block => BoxType::BlockNode(node_id),
                Display::Inline => BoxType::InlineNode(node_id),
            };

            // Detect replaced element (<img>)
            if dom.tag_name(node_id).is_some_and(|t| t.eq_ignore_ascii_case("img")) {
                let attr_w = dom.get_attribute(node_id, "width").and_then(|s| s.parse::<f32>().ok());
                let attr_h = dom.get_attribute(node_id, "height").and_then(|s| s.parse::<f32>().ok());

                if let Some(&(iw, ih)) = context.image_dimensions.get(&node_id) {
                    intrinsic_size = Some(nirvana_core::geometry::Size::new(iw, ih));
                } else if attr_w.is_some() || attr_h.is_some() {
                    let iw = attr_w.unwrap_or_else(|| attr_h.unwrap_or(16.0));
                    let ih = attr_h.unwrap_or_else(|| attr_w.unwrap_or(16.0));
                    intrinsic_size = Some(nirvana_core::geometry::Size::new(iw, ih));
                } else {
                    intrinsic_size = Some(nirvana_core::geometry::Size::new(16.0, 16.0));
                }
            }

            b_type
        }
        NodeType::Text => {
            // Check if text is non-empty / meaningful
            if let Some(node) = dom.get_node(node_id)
                && let NodeData::Text(ref text) = node.data
            {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let metrics = nirvana_font::Font::default_font().measure_text(trimmed, 16.0);
                initial_size = Some(nirvana_core::geometry::Size::new(metrics.width, metrics.line_height));
            } else {
                return None;
            }
            BoxType::InlineNode(node_id)
        }
        NodeType::Comment | NodeType::Document => return None,
    };

    let mut raw_children = Vec::new();
    for child in dom.children(node_id) {
        if let Some(child_box) = build_box(child, dom, style_tree, context) {
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
    layout_box.intrinsic_size = intrinsic_size;
    if let Some(size) = initial_size {
        layout_box.dimensions.content.size = size;
    }
    layout_box.children = final_children;
    Some(layout_box)
}
