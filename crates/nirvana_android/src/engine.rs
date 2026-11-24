//! Core browser engine state machine for Android embedding.

use crate::hit_test::{resolve_hit_target, HitTestResult};
use nirvana_core::geometry::{Point, Rect, Size};
use nirvana_css::ast::StyleSheet;
use nirvana_css::parser::parse_css;
use nirvana_dom::node::NodeData;
use nirvana_dom::{DomTree, NodeId, NodeType};
use nirvana_gfx::builder::{build_display_list_full, FormState, InputBuffer};
use nirvana_gfx::canvas::Canvas;
use nirvana_gfx::display_list::DisplayList;
use nirvana_gfx::image::{decode_image, ImageMap};
use nirvana_html::parse_html;
use nirvana_layout::box_type::LayoutBox;
use nirvana_layout::layout::layout_document_with_context;
use nirvana_layout::tree_builder::LayoutContext;
use nirvana_net::loader::ResourceLoader;
use nirvana_style::compute::{compute_styles, StyleTree};
use nirvana_url::Url;
use std::collections::HashMap;
use std::sync::Arc;

/// Orchestrates HTML parsing, CSS style computation, layout geometry,
/// and pixel rasterization into a 32-bit RGBA framebuffer.
#[derive(Debug)]
pub struct NirvanaEngine {
    width: u32,
    height: u32,
    dom: Option<DomTree>,
    stylesheets: Vec<StyleSheet>,
    style_tree: Option<StyleTree>,
    layout_root: Option<LayoutBox>,
    display_list: Option<DisplayList>,
    scroll_x: f32,
    scroll_y: f32,
    canvas: Canvas,
    loader: ResourceLoader,
    current_url: Option<String>,
    images: ImageMap,
    focused_element: Option<NodeId>,
    input_buffers: HashMap<NodeId, InputBuffer>,
}

impl NirvanaEngine {
    /// Creates a new `NirvanaEngine` instance with given viewport dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            dom: None,
            stylesheets: Vec::new(),
            style_tree: None,
            layout_root: None,
            display_list: None,
            scroll_x: 0.0,
            scroll_y: 0.0,
            canvas: Canvas::new(width, height),
            loader: ResourceLoader::new(),
            current_url: None,
            images: ImageMap::new(),
            focused_element: None,
            input_buffers: HashMap::new(),
        }
    }

    /// Returns the current viewport dimensions (width, height).
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns a direct byte slice reference to the 32-bit RGBA framebuffer pixels.
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.canvas.pixels
    }

    /// Returns the current loaded document URL string, if any.
    #[inline]
    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    /// Loads and parses an HTML document string, with optional user/author CSS.
    pub fn load_html(&mut self, html: &str, css: Option<&str>) {
        let dom = parse_html(html);
        self.images.clear();
        self.fetch_and_decode_images(&dom, None);
        self.dom = Some(dom);
        self.stylesheets.clear();
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;
        self.current_url = None;
        self.focused_element = None;
        self.input_buffers.clear();

        if let Some(ref d) = self.dom {
            let inline_sheets = discover_inline_stylesheets(d);
            self.stylesheets.extend(inline_sheets);
        }

        if let Some(css_text) = css {
            let sheet = parse_css(css_text);
            self.stylesheets.push(sheet);
        }
    }

    /// Navigates to a given URL string.
    /// Fetches document, extracts and fetches linked external stylesheets, and renders.
    pub fn navigate(&mut self, url_str: &str) -> Result<(), String> {
        let url = Url::parse(url_str).map_err(|e| e.to_string())?;
        let resp = self.loader.fetch(&url).map_err(|e| e.to_string())?;

        if resp.status < 200 || resp.status >= 300 {
            return Err(format!("HTTP error status: {}", resp.status));
        }

        let html = resp.text().map_err(|e| format!("Invalid UTF-8 document: {e}"))?;
        let dom = parse_html(html);

        // Discover and load linked external stylesheets: <link rel="stylesheet" href="...">
        let mut external_sheets = Vec::new();
        let linked_hrefs = discover_linked_stylesheets(&dom);
        for href in linked_hrefs {
            let css_url = match url.join(&href) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("Warning: failed to resolve stylesheet URL '{href}': {e}");
                    continue;
                }
            };

            let css_resp = match self.loader.fetch(&css_url) {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Warning: failed to fetch stylesheet at {css_url}: {e}");
                    continue;
                }
            };

            if css_resp.status < 200 || css_resp.status >= 300 {
                eprintln!(
                    "Warning: failed to load stylesheet at {}: HTTP {}",
                    css_url, css_resp.status
                );
                continue;
            }

            match css_resp.text() {
                Ok(css_text) => {
                    let sheet = parse_css(css_text);
                    external_sheets.push(sheet);
                }
                Err(e) => {
                    eprintln!("Warning: invalid UTF-8 in stylesheet at {css_url}: {e}");
                }
            }
        }

        // Discover and load embedded images: <img src="...">
        self.images.clear();
        self.fetch_and_decode_images(&dom, Some(&url));

        let inline_sheets = discover_inline_stylesheets(&dom);
        self.dom = Some(dom);
        self.stylesheets = external_sheets;
        self.stylesheets.extend(inline_sheets);
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;
        self.current_url = Some(url_str.to_string());
        self.focused_element = None;
        self.input_buffers.clear();

        self.render();
        Ok(())
    }

    /// Resizes the viewport and framebuffer canvas.
    /// Re-renders existing document if one is loaded.
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        if self.dom.is_some() {
            self.render();
            self.scroll_to(self.scroll_x, self.scroll_y);
        } else {
            self.canvas = Canvas::new(width, height);
        }
    }

    /// Executes the full styling, layout, and rasterization pipeline onto the canvas.
    pub fn render(&mut self) {
        let Some(ref dom) = self.dom else {
            return;
        };

        let sheets: Vec<&StyleSheet> = self.stylesheets.iter().collect();
        let style_tree = compute_styles(dom, &sheets);

        let mut layout_ctx = LayoutContext::default();
        for (node_id, src) in discover_images(dom) {
            if let Some(img) = self.images.get(&src) {
                layout_ctx
                    .image_dimensions
                    .insert(node_id, (img.width as f32, img.height as f32));
            }
        }

        let viewport = Size::new(self.width as f32, self.height as f32);
        let layout_root = layout_document_with_context(dom, &style_tree, viewport, &layout_ctx);

        let form_state = FormState {
            focused_node: self.focused_element,
            inputs: self.input_buffers.clone(),
        };

        let mut canvas = Canvas::new(self.width, self.height);
        let display_list = if let Some(ref root) = layout_root {
            let list = build_display_list_full(
                root,
                &style_tree,
                dom,
                &self.images,
                &form_state,
            );
            canvas.render_display_list_with_offset(&list, -self.scroll_x, -self.scroll_y);
            Some(list)
        } else {
            None
        };

        self.style_tree = Some(style_tree);
        self.layout_root = layout_root;
        self.display_list = display_list;
        self.canvas = canvas;
    }

    fn fetch_and_decode_images(&mut self, dom: &DomTree, base_url: Option<&Url>) {
        let discovered = discover_images(dom);
        for (_node_id, src) in discovered {
            if self.images.contains_key(&src) {
                continue;
            }

            let img_url = match base_url {
                Some(base) => match base.join(&src) {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("Warning: failed to resolve image URL '{src}': {e}");
                        continue;
                    }
                },
                None => match Url::parse(&src) {
                    Ok(u) => u,
                    Err(_) => continue,
                },
            };

            // Check if canonical URL is already cached
            let canonical = img_url.as_str();
            if let Some(existing) = self.images.get(&canonical) {
                self.images.insert(src.clone(), existing.clone());
                continue;
            }

            let resp = match self.loader.fetch(&img_url) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Warning: failed to fetch image at {img_url}: {e}");
                    continue;
                }
            };

            if resp.status < 200 || resp.status >= 300 {
                eprintln!(
                    "Warning: failed to load image at {}: HTTP {}",
                    img_url, resp.status
                );
                continue;
            }

            match decode_image(&resp.body) {
                Ok(decoded) => {
                    let decoded_arc = Arc::new(decoded);
                    self.images.insert(src.clone(), decoded_arc.clone());
                    self.images.insert(img_url.as_str(), decoded_arc);
                }
                Err(e) => {
                    eprintln!("Warning: failed to decode image at {img_url}: {e}");
                }
            }
        }
    }

    /// Sets the viewport scroll offset to the specified coordinates, clamped to document bounds.
    /// Re-rasterizes the display list with offset if scroll changed.
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        let (max_x, max_y) = self.max_scroll();
        let target_x = if x.is_finite() {
            x.clamp(0.0, max_x)
        } else {
            self.scroll_x
        };
        let target_y = if y.is_finite() {
            y.clamp(0.0, max_y)
        } else {
            self.scroll_y
        };

        if (self.scroll_x - target_x).abs() < f32::EPSILON
            && (self.scroll_y - target_y).abs() < f32::EPSILON
        {
            return;
        }

        self.scroll_x = target_x;
        self.scroll_y = target_y;

        if let Some(ref list) = self.display_list {
            self.canvas = Canvas::new(self.width, self.height);
            self.canvas
                .render_display_list_with_offset(list, -self.scroll_x, -self.scroll_y);
        }
    }

    /// Scrolls the viewport by the given delta (dx, dy), clamped to document bounds.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.scroll_to(self.scroll_x + dx, self.scroll_y + dy);
    }

    /// Returns current viewport scroll offset (scroll_x, scroll_y).
    #[inline]
    pub fn scroll_offset(&self) -> (f32, f32) {
        (self.scroll_x, self.scroll_y)
    }

    /// Returns the total scrollable document dimensions (width, height).
    pub fn document_dimensions(&self) -> (f32, f32) {
        if let Some(ref root) = self.layout_root {
            calculate_content_extent(root)
        } else {
            (self.width as f32, self.height as f32)
        }
    }

    /// Returns the maximum scroll offset (max_x, max_y).
    pub fn max_scroll(&self) -> (f32, f32) {
        let (doc_w, doc_h) = self.document_dimensions();
        let max_x = (doc_w - self.width as f32).max(0.0);
        let max_y = (doc_h - self.height as f32).max(0.0);
        (max_x, max_y)
    }

    /// Performs hit-testing at the given screen coordinates (screen_x, screen_y).
    /// Translates screen coordinates to document coordinates using current scroll offset.
    pub fn hit_test(&self, screen_x: f32, screen_y: f32) -> Option<HitTestResult> {
        let (root, dom) = (self.layout_root.as_ref()?, self.dom.as_ref()?);
        let doc_x = screen_x + self.scroll_x;
        let doc_y = screen_y + self.scroll_y;
        let hit_node_id = root.hit_test(Point::new(doc_x, doc_y))?;
        Some(resolve_hit_target(dom, hit_node_id))
    }

    /// Handles a click/touch tap at screen coordinates (screen_x, screen_y).
    /// If an interactive form control (<input> or <textarea>) is clicked,
    /// sets focus on it, calculates the cursor index from the tap coordinate,
    /// and triggers re-rendering. If a non-input element is clicked, blurs.
    pub fn handle_click(&mut self, screen_x: f32, screen_y: f32) -> Option<HitTestResult> {
        let hit = self.hit_test(screen_x, screen_y);
        if let Some(ref result) = hit {
            let is_form_input = result.tag.eq_ignore_ascii_case("input")
                || result.tag.eq_ignore_ascii_case("textarea");
            if is_form_input {
                let doc_x = screen_x + self.scroll_x;
                let rel_x = if let Some(ref root) = self.layout_root
                    && let Some(lbox) = find_box_for_node(root, result.node_id)
                {
                    (doc_x - lbox.dimensions.content.origin.x).max(0.0)
                } else {
                    0.0
                };

                let text = self.get_input_state(result.node_id).0;
                let is_password = self.dom.as_ref().is_some_and(|d| {
                    d.get_attribute(result.node_id, "type")
                        .is_some_and(|t| t.eq_ignore_ascii_case("password"))
                });
                let font = nirvana_font::Font::default_font();
                let mut cursor_idx = text.chars().count();
                let mut accum = 0.0;
                for (i, c) in text.chars().enumerate() {
                    let char_repr = if is_password {
                        "•".to_string()
                    } else {
                        c.to_string()
                    };
                    let w = font.measure_text(&char_repr, 16.0).width;
                    if rel_x < accum + (w / 2.0) {
                        cursor_idx = i;
                        break;
                    }
                    accum += w;
                }
                self.focus(result.node_id, cursor_idx);
            } else {
                self.blur();
            }
        } else {
            self.blur();
        }
        hit
    }

    /// Focuses the given form element and places the caret cursor at `cursor_index`.
    pub fn focus(&mut self, node_id: NodeId, cursor_index: usize) {
        let (text, _, sel) = self.get_input_state(node_id);
        let clamped_cursor = cursor_index.min(text.chars().count());
        self.input_buffers.insert(node_id, (text, clamped_cursor, sel));
        self.focused_element = Some(node_id);
        self.render();
    }

    /// Removes focus from currently focused element.
    pub fn blur(&mut self) {
        if self.focused_element.is_some() {
            self.focused_element = None;
            self.render();
        }
    }

    /// Returns `true` if an <input> or <textarea> element currently has focus.
    pub fn is_input_focused(&self) -> bool {
        if let Some(id) = self.focused_element
            && let Some(ref dom) = self.dom
            && let Some(tag) = dom.tag_name(id)
        {
            tag.eq_ignore_ascii_case("input") || tag.eq_ignore_ascii_case("textarea")
        } else {
            false
        }
    }

    /// Returns the NodeId of the currently focused element, if any.
    pub fn focused_element(&self) -> Option<NodeId> {
        self.focused_element
    }

    /// Returns the text content of the currently focused form element.
    pub fn focused_text(&self) -> Option<String> {
        let node_id = self.focused_element?;
        Some(self.get_input_state(node_id).0)
    }

    /// Returns the viewport screen bounds of the currently focused form element.
    pub fn focused_bounds(&self) -> Option<Rect> {
        let node_id = self.focused_element?;
        let root = self.layout_root.as_ref()?;
        let lbox = find_box_for_node(root, node_id)?;
        let border_box = lbox.dimensions.border_box();
        Some(Rect::new(
            border_box.x() - self.scroll_x,
            border_box.y() - self.scroll_y,
            border_box.width(),
            border_box.height(),
        ))
    }

    /// Inserts `text` into the focused form control at the current caret position,
    /// advances the caret, syncs DOM `value`, and re-renders.
    pub fn insert_text(&mut self, text: &str) {
        let Some(node_id) = self.focused_element else {
            return;
        };

        let (mut curr_text, mut cursor_idx, _sel) = self.get_input_state(node_id);

        let char_count = curr_text.chars().count();
        cursor_idx = cursor_idx.min(char_count);

        let byte_pos = curr_text
            .char_indices()
            .map(|(idx, _)| idx)
            .nth(cursor_idx)
            .unwrap_or(curr_text.len());

        curr_text.insert_str(byte_pos, text);
        cursor_idx += text.chars().count();

        if let Some(ref mut dom) = self.dom {
            let _ = dom.set_attribute(node_id, "value", &curr_text);
        }

        self.input_buffers.insert(node_id, (curr_text, cursor_idx, None));
        self.render();
    }

    /// Deletes the character preceding the caret in the focused form control,
    /// decrements the caret, syncs DOM `value`, and re-renders.
    pub fn delete_backward(&mut self) {
        let Some(node_id) = self.focused_element else {
            return;
        };

        let (mut curr_text, mut cursor_idx, _sel) = self.get_input_state(node_id);

        if cursor_idx == 0 || curr_text.is_empty() {
            return;
        }

        let char_count = curr_text.chars().count();
        cursor_idx = cursor_idx.min(char_count);

        let char_to_delete = cursor_idx - 1;
        let byte_start = curr_text
            .char_indices()
            .map(|(idx, _)| idx)
            .nth(char_to_delete)
            .unwrap_or(0);
        let byte_end = curr_text
            .char_indices()
            .map(|(idx, _)| idx)
            .nth(cursor_idx)
            .unwrap_or(curr_text.len());

        curr_text.replace_range(byte_start..byte_end, "");
        cursor_idx -= 1;

        if let Some(ref mut dom) = self.dom {
            let _ = dom.set_attribute(node_id, "value", &curr_text);
        }

        self.input_buffers.insert(node_id, (curr_text, cursor_idx, None));
        self.render();
    }

    /// Sets the caret cursor position in the focused form control.
    pub fn set_cursor_index(&mut self, index: usize) {
        let Some(node_id) = self.focused_element else {
            return;
        };
        let (text, _, sel) = self.get_input_state(node_id);
        let clamped = index.min(text.chars().count());
        self.input_buffers.insert(node_id, (text, clamped, sel));
        self.render();
    }

    /// Sets the text selection range in the focused form control.
    pub fn set_selection(&mut self, start: usize, end: usize) {
        let Some(node_id) = self.focused_element else {
            return;
        };
        let (text, cursor, _) = self.get_input_state(node_id);
        let max_len = text.chars().count();
        let s = start.min(max_len);
        let e = end.min(max_len);
        self.input_buffers.insert(node_id, (text, cursor, Some((s, e))));
        self.render();
    }

    fn get_input_state(&self, node_id: NodeId) -> (String, usize, Option<(usize, usize)>) {
        if let Some(state) = self.input_buffers.get(&node_id) {
            state.clone()
        } else if let Some(ref dom) = self.dom {
            if let Some(val) = dom.get_attribute(node_id, "value") {
                (val.to_string(), val.chars().count(), None)
            } else {
                let mut child_text = String::new();
                for child in dom.children(node_id) {
                    if dom.node_type(child) == Some(NodeType::Text)
                        && let Some(node) = dom.get_node(child)
                        && let NodeData::Text(ref t) = node.data
                    {
                        child_text.push_str(t);
                    }
                }
                let len = child_text.chars().count();
                (child_text, len, None)
            }
        } else {
            (String::new(), 0, None)
        }
    }
}

fn find_box_for_node(layout_box: &LayoutBox, target_id: NodeId) -> Option<&LayoutBox> {
    if layout_box.node_id() == Some(target_id) {
        return Some(layout_box);
    }
    for child in &layout_box.children {
        if let Some(found) = find_box_for_node(child, target_id) {
            return Some(found);
        }
    }
    None
}

fn calculate_content_extent(layout_box: &LayoutBox) -> (f32, f32) {
    let mb = layout_box.dimensions.margin_box();
    let mut max_x = mb.right();
    let mut max_y = mb.bottom();
    for child in &layout_box.children {
        let (cx, cy) = calculate_content_extent(child);
        max_x = max_x.max(cx);
        max_y = max_y.max(cy);
    }
    (max_x, max_y)
}

fn discover_linked_stylesheets(dom: &DomTree) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && let Some(tag) = dom.tag_name(node_id)
            && tag.eq_ignore_ascii_case("link")
            && let Some(rel) = dom.get_attribute(node_id, "rel")
            && rel.to_ascii_lowercase().contains("stylesheet")
            && let Some(href) = dom.get_attribute(node_id, "href")
        {
            hrefs.push(href.to_string());
        }
        for child in dom.children(node_id).into_iter().rev() {
            stack.push(child);
        }
    }
    hrefs
}

fn discover_images(dom: &DomTree) -> Vec<(NodeId, String)> {
    let mut images = Vec::new();
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && let Some(tag) = dom.tag_name(node_id)
            && tag.eq_ignore_ascii_case("img")
            && let Some(src) = dom.get_attribute(node_id, "src")
            && !src.trim().is_empty()
        {
            images.push((node_id, src.trim().to_string()));
        }
        for child in dom.children(node_id).into_iter().rev() {
            stack.push(child);
        }
    }
    images
}

fn discover_inline_stylesheets(dom: &DomTree) -> Vec<StyleSheet> {
    let mut sheets = Vec::new();
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && let Some(tag) = dom.tag_name(node_id)
            && tag.eq_ignore_ascii_case("style")
        {
            let mut css_text = String::new();
            for child in dom.children(node_id) {
                if dom.node_type(child) == Some(NodeType::Text)
                    && let Some(node) = dom.get_node(child)
                    && let NodeData::Text(ref text) = node.data
                {
                    css_text.push_str(text);
                }
            }
            if !css_text.trim().is_empty() {
                sheets.push(parse_css(&css_text));
            }
        }
        for child in dom.children(node_id).into_iter().rev() {
            stack.push(child);
        }
    }
    sheets
}
