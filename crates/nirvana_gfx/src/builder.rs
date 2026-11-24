//! Constructs a DisplayList from a positioned LayoutBox tree according to CSS 2.1 painting order.

use crate::display_list::{DisplayCommand, DisplayList};
use crate::image::ImageMap;
use nirvana_core::color::Rgba;
use nirvana_core::geometry::Rect;
use nirvana_dom::node::NodeData;
use nirvana_dom::{DomTree, NodeId, NodeType};
use nirvana_layout::box_type::{BoxType, LayoutBox};
use nirvana_layout::dimensions::EdgeSizes;
use nirvana_style::compute::StyleTree;
use std::collections::HashMap;

/// Form control text state tuple: (current_text, caret_char_index, Option<selection_range>).
pub type InputBuffer = (String, usize, Option<(usize, usize)>);

/// State of interactive form controls (focused element, text buffers, caret positions).
#[derive(Debug, Clone, Default)]
pub struct FormState {
    pub focused_node: Option<NodeId>,
    /// NodeId -> (current_text, caret_char_index, Option<selection_range>)
    pub inputs: HashMap<NodeId, InputBuffer>,
}

/// Builds a `DisplayList` by traversing the layout tree in CSS 2.1 Appendix E painting order.
pub fn build_display_list(
    layout_root: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
) -> DisplayList {
    build_display_list_full(layout_root, style_tree, dom, &ImageMap::new(), &FormState::default())
}

/// Builds a `DisplayList` including decoded image assets from the provided `ImageMap`.
pub fn build_display_list_with_images(
    layout_root: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
    images: &ImageMap,
) -> DisplayList {
    build_display_list_full(layout_root, style_tree, dom, images, &FormState::default())
}

/// Builds a `DisplayList` including decoded images and interactive form control state.
pub fn build_display_list_full(
    layout_root: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
    images: &ImageMap,
    form_state: &FormState,
) -> DisplayList {
    let mut list = DisplayList::new();
    render_layout_box(layout_root, style_tree, dom, images, form_state, &mut list);
    list
}

fn render_layout_box(
    layout_box: &LayoutBox,
    style_tree: &StyleTree,
    dom: &DomTree,
    images: &ImageMap,
    form_state: &FormState,
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
    {
        let img_opt = dom.get_attribute(id, "src").and_then(|s| images.get(s));
        if let Some(img) = img_opt {
            list.push(DisplayCommand::DrawImage {
                rect: layout_box.dimensions.content,
                image: img.clone(),
            });
        } else {
            // Broken or missing image placeholder border
            list.push(DisplayCommand::Border {
                rect: layout_box.dimensions.content,
                widths: EdgeSizes::new(1.0, 1.0, 1.0, 1.0),
                color: Rgba::new(200, 200, 200, 255),
            });
            if let Some(alt) = dom.get_attribute(id, "alt")
                && !alt.is_empty()
            {
                list.push(DisplayCommand::Text {
                    text: alt.to_string(),
                    origin: layout_box.dimensions.content.origin,
                    color: Rgba::new(100, 100, 100, 255),
                });
            }
        }
    }

    // 4. Paint Form Controls: <input>, <textarea>, <button>
    if let Some(id) = layout_box.node_id()
        && dom.node_type(id) == Some(NodeType::Element)
        && let Some(tag) = dom.tag_name(id)
    {
        let is_input = tag.eq_ignore_ascii_case("input");
        let is_textarea = tag.eq_ignore_ascii_case("textarea");
        let is_button = tag.eq_ignore_ascii_case("button");

        if is_input || is_textarea || is_button {
            let is_focused = form_state.focused_node == Some(id);

            // Active focus ring / outline
            if is_focused {
                list.push(DisplayCommand::Border {
                    rect: layout_box.dimensions.border_box(),
                    widths: EdgeSizes::new(2.0, 2.0, 2.0, 2.0),
                    color: Rgba::new(0, 122, 255, 255), // Active blue focus outline
                });
            }

            // Text extraction & masking
            let (raw_text, cursor_idx, selection_opt) = if let Some(state) = form_state.inputs.get(&id) {
                (state.0.clone(), state.1, state.2)
            } else if let Some(val) = dom.get_attribute(id, "value") {
                (val.to_string(), val.chars().count(), None)
            } else {
                let mut child_text = String::new();
                for child in dom.children(id) {
                    if dom.node_type(child) == Some(NodeType::Text)
                        && let Some(node) = dom.get_node(child)
                        && let NodeData::Text(ref t) = node.data
                    {
                        child_text.push_str(t);
                    }
                }
                let len = child_text.chars().count();
                (child_text, len, None)
            };

            let is_password = is_input && dom.get_attribute(id, "type").is_some_and(|t| t.eq_ignore_ascii_case("password"));
            let display_text = if is_password {
                "•".repeat(raw_text.chars().count())
            } else {
                raw_text
            };

            let font = nirvana_font::Font::default_font();
            let text_color = style.map(|s| s.color).unwrap_or_else(|| Rgba::new(0, 0, 0, 255));

            // Selection Highlight
            if let Some((sel_start, sel_end)) = selection_opt {
                let start = sel_start.min(sel_end);
                let end = sel_start.max(sel_end);
                if start != end {
                    let pre_str: String = display_text.chars().take(start).collect();
                    let sel_str: String = display_text.chars().skip(start).take(end - start).collect();
                    let sel_x = layout_box.dimensions.content.origin.x + font.measure_text(&pre_str, 14.0).width;
                    let sel_w = font.measure_text(&sel_str, 14.0).width;
                    list.push(DisplayCommand::TextSelection {
                        rect: Rect::new(sel_x, layout_box.dimensions.content.origin.y, sel_w, layout_box.dimensions.content.height()),
                        color: Rgba::new(180, 215, 255, 128),
                    });
                }
            }

            // Text / Placeholder content
            if !display_text.is_empty() {
                list.push(DisplayCommand::Text {
                    text: display_text.clone(),
                    origin: layout_box.dimensions.content.origin,
                    color: text_color,
                });
            } else if let Some(placeholder) = dom.get_attribute(id, "placeholder")
                && !placeholder.is_empty()
            {
                list.push(DisplayCommand::Text {
                    text: placeholder.to_string(),
                    origin: layout_box.dimensions.content.origin,
                    color: Rgba::new(150, 150, 150, 255), // Dim placeholder gray
                });
            }

            // Blinking Caret cursor if focused
            if is_focused && !is_button {
                let bounded_idx = cursor_idx.min(display_text.chars().count());
                let prefix: String = display_text.chars().take(bounded_idx).collect();
                let caret_x = layout_box.dimensions.content.origin.x + font.measure_text(&prefix, 14.0).width;
                let caret_y = layout_box.dimensions.content.origin.y + 2.0;
                let caret_h = (layout_box.dimensions.content.height() - 4.0).max(14.0);

                list.push(DisplayCommand::Caret {
                    rect: Rect::new(caret_x, caret_y, 1.5, caret_h),
                    color: text_color,
                });
            }
        }
    }

    // 5. Paint text content if this is an inline text node
    if let BoxType::InlineNode(id) = layout_box.box_type
        && dom.node_type(id) == Some(NodeType::Text)
        && let Some(node) = dom.get_node(id)
        && let NodeData::Text(ref text) = node.data
    {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            // Text nodes are not in StyleTree; walk ancestors to resolve computed color
            let mut text_color = Rgba::new(0, 0, 0, 255);
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

    // 6. Paint children recursively in tree order (skip replaced form controls)
    let is_replaced_form = layout_box.node_id().is_some_and(|id| {
        dom.tag_name(id).is_some_and(|t| {
            t.eq_ignore_ascii_case("input") || t.eq_ignore_ascii_case("textarea")
        })
    });
    if !is_replaced_form {
        for child in &layout_box.children {
            render_layout_box(child, style_tree, dom, images, form_state, list);
        }
    }
}
