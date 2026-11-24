//! Constructs a DisplayList from a positioned LayoutBox tree according to CSS 2.1 painting order.

use crate::display_list::{DisplayCommand, DisplayList};
use crate::image::ImageMap;
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
    build_display_list_with_images(layout_root, style_tree, dom, &ImageMap::new())
}

/// Builds a `DisplayList` including decoded image assets from the provided `ImageMap`.
pub fn build_display_list_with_images(
    layout_root: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
    images: &ImageMap,
) -> DisplayList {
    let mut list = DisplayList::new();
    render_layout_box(layout_root, style_tree, dom, images, &mut list);
    list
}

fn render_layout_box(
    layout_box: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
    images: &ImageMap,
    list: &mut DisplayList,
) {
    let style = layout_box
        .node_id()
        .and_then(|id| style_tree.get_style(id));

    // 1. Paint background of current box (covers border_box per CSS 2.1 / CSS Backgrounds 3)
    if let Some(s) = style {
        if s.background_color.a > 0 {
            list.push(DisplayCommand::SolidColor {
                rect: layout_box.dimensions.border_box(),
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

    // 3. Paint image if this is an <img> element with decoded image asset
    if let Some(id) = layout_box.node_id()
        && dom.node_type(id) == Some(NodeType::Element)
        && dom.tag_name(id).is_some_and(|t| t.eq_ignore_ascii_case("img"))
        && let Some(src) = dom.get_attribute(id, "src")
    {
        if let Some(img) = images.get(src) {
            list.push(DisplayCommand::DrawImage {
                rect: layout_box.dimensions.content,
                image: img.clone(),
            });
        } else {
            // Broken image placeholder border
            list.push(DisplayCommand::Border {
                rect: layout_box.dimensions.content,
                widths: nirvana_layout::dimensions::EdgeSizes::new(1.0, 1.0, 1.0, 1.0),
                color: nirvana_core::color::Rgba::new(200, 200, 200, 255),
            });
            if let Some(alt) = dom.get_attribute(id, "alt")
                && !alt.is_empty()
            {
                list.push(DisplayCommand::Text {
                    text: alt.to_string(),
                    origin: layout_box.dimensions.content.origin,
                    color: nirvana_core::color::Rgba::new(100, 100, 100, 255),
                });
            }
        }
    }

    // 4. Paint text content if this is an inline text node
    if let BoxType::InlineNode(id) = layout_box.box_type
        && dom.node_type(id) == Some(NodeType::Text)
        && let Some(node) = dom.get_node(id)
        && let NodeData::Text(ref text) = node.data
    {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            // Text nodes are not in StyleTree; walk ancestors to resolve computed color
            let mut text_color = nirvana_core::color::Rgba::new(0, 0, 0, 255);
            let mut curr = Some(id);
            while let Some(node_id) = curr {
                if let Some(s) = style_tree.get_style(node_id) {
                    text_color = s.color;
                    break;
                }
                curr = dom.parent(node_id);
            }

            list.push(DisplayCommand::Text {
                text: trimmed.to_string(),
                origin: layout_box.dimensions.content.origin,
                color: text_color,
            });
        }
    }

    // 5. Paint children recursively in tree order
    for child in &layout_box.children {
        render_layout_box(child, style_tree, dom, images, list);
    }
}
