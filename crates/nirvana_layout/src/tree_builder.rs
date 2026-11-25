//! Layout tree construction from styled DOM nodes.

use crate::box_type::{BoxType, LayoutBox};
use nirvana_core::arena::Id;
use nirvana_dom::node::{Node, NodeData};
use nirvana_dom::{DomTree, NodeType};
use nirvana_style::compute::StyleTree;
use nirvana_style::values::{Display, Position};

use std::collections::HashMap;

/// Context for layout tree construction containing asset information such as image dimensions.
#[derive(Debug, Default, Clone)]
pub struct LayoutContext {
    /// Intrinsic dimensions (width, height) for image element nodes.
    pub image_dimensions: HashMap<Id<Node>, (f32, f32)>,
    /// Viewport width for responsive line wrapping.
    pub viewport_width: f32,
}

/// Collapses runs of HTML ASCII whitespace into single spaces while preserving edge presence.
pub fn collapse_html_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_ws = false;
    for ch in text.chars() {
        if ch.is_ascii_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out
}

/// Wraps text into lines based on word boundaries and maximum allowed width.
pub fn wrap_text_into_lines(
    text: &str,
    font: &nirvana_font::Font,
    font_size: f32,
    max_width: f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let effective_max = max_width.max(40.0);

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if font.measure_text(word, font_size).width <= effective_max {
                current_line.push_str(word);
            } else {
                // Break overlong token (e.g. URL) cleanly across lines
                for part in split_overlong_word(word, font, font_size, effective_max) {
                    if !current_line.is_empty() {
                        lines.push(std::mem::take(&mut current_line));
                    }
                    current_line = part;
                }
            }
        } else {
            let mut test_line = current_line.clone();
            test_line.push(' ');
            test_line.push_str(word);
            let w = font.measure_text(&test_line, font_size).width;
            if w <= effective_max {
                current_line = test_line;
            } else {
                lines.push(current_line);
                if font.measure_text(word, font_size).width <= effective_max {
                    current_line = word.to_string();
                } else {
                    let mut parts = split_overlong_word(word, font, font_size, effective_max);
                    current_line = parts.pop().unwrap_or_default();
                    lines.extend(parts);
                }
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(text.to_string());
    }
    lines
}

pub(crate) fn split_overlong_word(
    word: &str,
    font: &nirvana_font::Font,
    font_size: f32,
    max_width: f32,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut chunk = String::new();
    for ch in word.chars() {
        let mut next = chunk.clone();
        next.push(ch);
        if !chunk.is_empty() && font.measure_text(&next, font_size).width > max_width {
            parts.push(std::mem::take(&mut chunk));
        }
        chunk.push(ch);
    }
    if !chunk.is_empty() {
        parts.push(chunk);
    }
    parts
}

/// Builds the root of the layout tree from the given DOM tree and computed style tree.
/// Elements with `display: none` (like `<head>`, `<script>`, `<style>`) are omitted.
pub fn build_layout_tree(dom: &DomTree, style_tree: &StyleTree) -> Option<LayoutBox> {
    build_layout_tree_with_context(dom, style_tree, &LayoutContext::default())
}

const MAX_LAYOUT_DEPTH: usize = 256;

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

    build_box(root_element, dom, style_tree, context, 0)
}

fn build_box(
    node_id: Id<Node>,
    dom: &DomTree,
    style_tree: &StyleTree,
    context: &LayoutContext,
    depth: usize,
) -> Option<LayoutBox> {
    if depth >= MAX_LAYOUT_DEPTH {
        return None;
    }
    let node_type = dom.node_type(node_id)?;
    let mut initial_size = None;
    let mut intrinsic_size = None;
    let mut display_mode = Display::Inline;
    let mut position_mode = Position::Static;
    let mut text_content = None;
    let mut is_line_break = false;

    let box_type = match node_type {
        NodeType::Element => {
            let style = style_tree.get_style(node_id)?;
            display_mode = style.display;
            position_mode = style.position;
            if display_mode == Display::None || style.offscreen_hidden {
                return None;
            }

            let tag_opt = dom.tag_name(node_id);
            if let Some(tag) = tag_opt {
                if tag.eq_ignore_ascii_case("input")
                    && dom
                        .get_attribute(node_id, "type")
                        .is_some_and(|t| t.eq_ignore_ascii_case("hidden"))
                {
                    return None;
                }
                if tag.eq_ignore_ascii_case("br") {
                    is_line_break = true;
                }
            }

            let b_type = match display_mode {
                Display::None => return None,
                Display::Block
                | Display::ListItem
                | Display::Flex
                | Display::Table
                | Display::TableRowGroup
                | Display::TableRow
                | Display::TableCell => BoxType::BlockNode(node_id),
                Display::Inline | Display::InlineBlock => BoxType::InlineNode(node_id),
            };

            // Detect replaced elements (<img>, <input>, <textarea>, <button>, <select>)
            if let Some(tag) = tag_opt {
                if tag.eq_ignore_ascii_case("img") {
                    let attr_w = dom.get_attribute(node_id, "width").and_then(parse_html_dim);
                    let attr_h = dom.get_attribute(node_id, "height").and_then(parse_html_dim);

                    let (iw, ih) = if let Some(&(w, h)) = context.image_dimensions.get(&node_id) {
                        (w, h)
                    } else {
                        (16.0, 16.0)
                    };

                    let effective_w = attr_w.unwrap_or_else(|| {
                        if let Some(ah) = attr_h {
                            if ih > 0.0 { ah * (iw / ih) } else { iw }
                        } else {
                            iw
                        }
                    });

                    let effective_h = attr_h.unwrap_or_else(|| {
                        if let Some(aw) = attr_w {
                            if iw > 0.0 { aw * (ih / iw) } else { ih }
                        } else {
                            ih
                        }
                    });

                    intrinsic_size = Some(nirvana_core::geometry::Size::new(effective_w, effective_h));
                } else if tag.eq_ignore_ascii_case("input") {
                    let input_type = dom
                        .get_attribute(node_id, "type")
                        .map(|t| t.to_ascii_lowercase())
                        .unwrap_or_default();
                    let attr_w = dom.get_attribute(node_id, "width").and_then(parse_html_dim);
                    let attr_h = dom.get_attribute(node_id, "height").and_then(parse_html_dim);

                    let (default_w, default_h) = match input_type.as_str() {
                        "submit" | "button" | "reset" => {
                            let label = dom.get_attribute(node_id, "value").unwrap_or("Submit");
                            let text_w = nirvana_font::Font::default_font()
                                .measure_text(label, 15.0)
                                .width;
                            ((text_w + 16.0).max(80.0), 26.0)
                        }
                        "checkbox" | "radio" => (16.0, 16.0),
                        _ => {
                            let size_cols = dom.get_attribute(node_id, "size").and_then(|s| s.parse::<f32>().ok());
                            (size_cols.map(|c| (c * 8.0).max(30.0)).unwrap_or(160.0), 24.0)
                        }
                    };

                    intrinsic_size = Some(nirvana_core::geometry::Size::new(
                        attr_w.unwrap_or(default_w),
                        attr_h.unwrap_or(default_h),
                    ));
                } else if tag.eq_ignore_ascii_case("textarea") {
                    let attr_w = dom.get_attribute(node_id, "width").and_then(parse_html_dim);
                    let attr_h = dom.get_attribute(node_id, "height").and_then(parse_html_dim);
                    let cols = dom.get_attribute(node_id, "cols").and_then(|s| s.parse::<f32>().ok());
                    let rows = dom.get_attribute(node_id, "rows").and_then(|s| s.parse::<f32>().ok());

                    let default_w = cols.map(|c| (c * 8.0).max(40.0)).unwrap_or(200.0);
                    let default_h = rows.map(|r| (r * 18.0).max(24.0)).unwrap_or(60.0);

                    intrinsic_size = Some(nirvana_core::geometry::Size::new(
                        attr_w.unwrap_or(default_w),
                        attr_h.unwrap_or(default_h),
                    ));
                } else if tag.eq_ignore_ascii_case("button") {
                    let attr_w = dom.get_attribute(node_id, "width").and_then(parse_html_dim);
                    let attr_h = dom.get_attribute(node_id, "height").and_then(parse_html_dim);
                    let label = collect_descendant_text(dom, node_id);
                    let text_w = nirvana_font::Font::default_font()
                        .measure_text(label.trim(), style.font_size)
                        .width;
                    let default_w = (text_w + 16.0).max(80.0);

                    intrinsic_size = Some(nirvana_core::geometry::Size::new(
                        attr_w.unwrap_or(default_w),
                        attr_h.unwrap_or(24.0),
                    ));
                } else if tag.eq_ignore_ascii_case("select") {
                    let selected_label = find_selected_option_text(dom, node_id);
                    let label = if selected_label.is_empty() {
                        "Select...".to_string()
                    } else {
                        format!("{selected_label} v")
                    };
                    let text_w = nirvana_font::Font::default_font()
                        .measure_text(&label, style.font_size)
                        .width;
                    text_content = Some(label);
                    intrinsic_size = Some(nirvana_core::geometry::Size::new(
                        (text_w + 16.0).clamp(90.0, 220.0),
                        26.0,
                    ));
                }
            }

            b_type
        }
        NodeType::Text => {
            if let Some(node) = dom.get_node(node_id)
                && let NodeData::Text(ref text) = node.data
            {
                let collapsed = collapse_html_whitespace(text);
                if collapsed.is_empty() {
                    return None;
                }
                let trimmed = collapsed.trim();
                let keep_space_only =
                    collapsed == " " && should_keep_single_space(dom, style_tree, node_id);
                if trimmed.is_empty() && !keep_space_only {
                    return None;
                }

                let mut font_size = 16.0;
                let mut curr = dom.parent(node_id);
                while let Some(pid) = curr {
                    if let Some(s) = style_tree.get_style(pid) {
                        font_size = s.font_size;
                        break;
                    }
                    curr = dom.parent(pid);
                }

                let font = nirvana_font::Font::default_font();
                let max_w = if context.viewport_width > 40.0 {
                    context.viewport_width - 24.0
                } else {
                    360.0
                };
                let lines = wrap_text_into_lines(if trimmed.is_empty() { " " } else { &collapsed }, font, font_size, max_w);
                let line_height = font.measure_text("Ay", font_size).line_height.max(font_size * 1.25);
                let max_line_w = lines
                    .iter()
                    .map(|l| font.measure_text(l, font_size).width)
                    .fold(0.0, f32::max);
                let total_h = (lines.len() as f32) * line_height;

                text_content = Some(collapsed);
                initial_size = Some(nirvana_core::geometry::Size::new(max_line_w, total_h));
            } else {
                return None;
            }
            BoxType::InlineNode(node_id)
        }
        NodeType::Comment | NodeType::Document => return None,
    };

    let skip_children = dom.tag_name(node_id).is_some_and(|t| {
        t.eq_ignore_ascii_case("select") || t.eq_ignore_ascii_case("textarea")
    });

    let mut raw_children = Vec::new();
    if !skip_children {
        for child in dom.children(node_id) {
            if let Some(child_box) = build_box(child, dom, style_tree, context, depth + 1) {
                raw_children.push(child_box);
            }
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
    layout_box.display_mode = display_mode;
    layout_box.position_mode = position_mode;
    layout_box.intrinsic_size = intrinsic_size;
    layout_box.text_content = text_content;
    layout_box.is_line_break = is_line_break;
    if let Some(size) = initial_size {
        layout_box.dimensions.content.size = size;
    }
    layout_box.children = final_children;
    Some(layout_box)
}

fn parse_html_dim(val: &str) -> Option<f32> {
    val.trim().strip_suffix("px").unwrap_or(val.trim()).parse::<f32>().ok()
}

fn collect_descendant_text(dom: &DomTree, node_id: Id<Node>) -> String {
    let mut s = String::new();
    for child in dom.children(node_id) {
        match dom.node_type(child) {
            Some(NodeType::Text) => {
                if let Some(n) = dom.get_node(child)
                    && let NodeData::Text(ref t) = n.data
                {
                    s.push_str(t);
                }
            }
            Some(NodeType::Element) => {
                s.push_str(&collect_descendant_text(dom, child));
            }
            _ => {}
        }
    }
    s
}

fn find_selected_option_text(dom: &DomTree, select_id: Id<Node>) -> String {
    let mut first_opt = None;
    let mut stack = dom.children(select_id);
    stack.reverse();
    while let Some(cid) = stack.pop() {
        if dom.tag_name(cid).is_some_and(|t| t.eq_ignore_ascii_case("option")) {
            let txt = collect_descendant_text(dom, cid).trim().to_string();
            if dom.get_attribute(cid, "selected").is_some() {
                return txt;
            }
            if first_opt.is_none() && !txt.is_empty() {
                first_opt = Some(txt);
            }
        } else {
            let mut kids = dom.children(cid);
            kids.reverse();
            stack.extend(kids);
        }
    }
    first_opt.unwrap_or_default()
}

/// Returns whether a sibling node participates as visible inline content for whitespace preservation.
fn is_inline_content_sibling(dom: &DomTree, style_tree: &StyleTree, sib_id: Id<Node>) -> bool {
    match dom.node_type(sib_id) {
        Some(NodeType::Element) => {
            if dom
                .tag_name(sib_id)
                .is_some_and(|t| t.eq_ignore_ascii_case("br"))
            {
                return false;
            }
            style_tree
                .get_style(sib_id)
                .is_some_and(|s| matches!(s.display, Display::Inline | Display::InlineBlock))
        }
        Some(NodeType::Text) => dom.get_node(sib_id).is_some_and(|n| match n.data {
            NodeData::Text(ref t) => !t.trim().is_empty(),
            _ => false,
        }),
        _ => false,
    }
}

/// Preserves collapsed single-space text nodes between adjacent inline siblings even inside block/table cells.
fn should_keep_single_space(dom: &DomTree, style_tree: &StyleTree, node_id: Id<Node>) -> bool {
    let Some(pid) = dom.parent(node_id) else {
        return false;
    };
    let parent_inline = style_tree
        .get_style(pid)
        .is_some_and(|s| matches!(s.display, Display::Inline | Display::InlineBlock));
    if parent_inline {
        return true;
    }

    let siblings = dom.children(pid);
    let Some(pos) = siblings.iter().position(|&id| id == node_id) else {
        return false;
    };

    let prev = siblings[..pos]
        .iter()
        .rev()
        .copied()
        .find(|&id| dom.node_type(id) != Some(NodeType::Comment));
    let next = siblings[pos + 1..]
        .iter()
        .copied()
        .find(|&id| dom.node_type(id) != Some(NodeType::Comment));

    matches!(
        (prev, next),
        (Some(p), Some(n))
            if is_inline_content_sibling(dom, style_tree, p)
                && is_inline_content_sibling(dom, style_tree, n)
    )
}
