//! Constructs a DisplayList from a positioned LayoutBox tree according to CSS 2.1 painting order.

use crate::display_list::{DisplayCommand, DisplayList};
use nirvana_dom::node::NodeData;
use nirvana_dom::{DomTree, NodeType};
use nirvana_layout::box_type::{BoxType, LayoutBox};
use nirvana_style::compute::StyleTree;

/// Builds a `DisplayList` by traversing the layout tree in CSS 2.1 Appendix E painting order.
pub fn build_display_list(
    layout_root: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
) -> DisplayList {
    let mut list = DisplayList::new();
    render_layout_box(layout_root, style_tree, dom, &mut list);
    list
}

fn render_layout_box(
    layout_box: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
    list: &mut DisplayList,
) {
    let style = layout_box
        .node_id()
        .and_then(|id| style_tree.get_style(id));

    // 1. Paint background of current box (covers padding_box)
    if let Some(s) = style {
        if s.background_color.a > 0 {
            list.push(DisplayCommand::SolidColor {
                rect: layout_box.dimensions.padding_box(),
                color: s.background_color,
            });
        }

        // 2. Paint borders around border_box
        if s.border_width > 0.0 {
            let b = layout_box.dimensions.border;
            if b.top > 0.0 || b.right > 0.0 || b.bottom > 0.0 || b.left > 0.0 {
                list.push(DisplayCommand::Border {
                    rect: layout_box.dimensions.border_box(),
                    widths: b,
                    color: s.color,
                });
            }
        }
    }

    // 3. Paint text content if this is an inline text node
    if let BoxType::InlineNode(id) = layout_box.box_type
        && dom.node_type(id) == Some(NodeType::Text)
        && let Some(node) = dom.get_node(id)
        && let NodeData::Text(ref text) = node.data
    {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let text_color = style
                .map(|s| s.color)
                .unwrap_or(nirvana_core::color::Rgba::new(0, 0, 0, 255));

            list.push(DisplayCommand::Text {
                text: trimmed.to_string(),
                origin: layout_box.dimensions.content.origin,
                color: text_color,
            });
        }
    }

    // 4. Paint children recursively in tree order
    for child in &layout_box.children {
        render_layout_box(child, style_tree, dom, list);
    }
}
