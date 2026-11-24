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

/// High-resolution performance metrics for individual rendering lifecycle stages.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameMetrics {
    pub parse_time_ms: f32,
    pub style_time_ms: f32,
    pub layout_time_ms: f32,
    pub raster_time_ms: f32,
    pub total_frame_time_ms: f32,
}

/// Orchestrates HTML parsing, CSS style computation, layout geometry,
/// and pixel rasterization into a 32-bit RGBA framebuffer.
#[derive(Debug)]
pub struct NirvanaEngine {
    width: u32,
    height: u32,
    dom: Option<DomTree>,
    stylesheets: Vec<StyleSheet>,
    external_stylesheets: Vec<StyleSheet>,
    author_css: Option<StyleSheet>,
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
    shared_dom: Option<std::rc::Rc<std::cell::RefCell<DomTree>>>,
    js_runtime: Option<nirvana_js::JsRuntime>,
    cookie_jar: std::rc::Rc<std::cell::RefCell<nirvana_net::cookie::CookieJar>>,
    storage_backend: std::rc::Rc<std::cell::RefCell<nirvana_js::storage::StorageBackend>>,
    csp_policy: Option<nirvana_net::security::CspPolicy>,
    last_metrics: FrameMetrics,
    adblock: std::rc::Rc<std::cell::RefCell<nirvana_adblock::AdBlockEngine>>,
}

impl NirvanaEngine {
    /// Creates a new `NirvanaEngine` instance with given viewport dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.clamp(1, 16384);
        let height = height.clamp(1, 16384);
        Self {
            width,
            height,
            dom: None,
            stylesheets: Vec::new(),
            external_stylesheets: Vec::new(),
            author_css: None,
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
            shared_dom: None,
            js_runtime: None,
            cookie_jar: std::rc::Rc::new(std::cell::RefCell::new(nirvana_net::cookie::CookieJar::new())),
            storage_backend: std::rc::Rc::new(std::cell::RefCell::new(nirvana_js::storage::StorageBackend::new())),
            csp_policy: None,
            last_metrics: FrameMetrics::default(),
            adblock: std::rc::Rc::new(std::cell::RefCell::new(nirvana_adblock::AdBlockEngine::new())),
        }
    }

    /// Returns reference to the sovereign AdBlock engine.
    #[inline]
    pub fn adblock_engine(&self) -> std::rc::Rc<std::cell::RefCell<nirvana_adblock::AdBlockEngine>> {
        self.adblock.clone()
    }

    /// Returns the total number of blocked ad and tracker requests.
    #[inline]
    pub fn blocked_requests_count(&self) -> u64 {
        self.adblock.borrow().blocked_count()
    }

    /// Enables or disables ad blocking globally.
    #[inline]
    pub fn set_adblock_enabled(&mut self, enabled: bool) {
        self.adblock.borrow_mut().set_enabled(enabled);
    }

    /// Returns true if ad blocking is currently active.
    #[inline]
    pub fn is_adblock_enabled(&self) -> bool {
        self.adblock.borrow().is_enabled()
    }

    /// Dynamically parses and registers a custom AdBlock Plus / uBlock filter rule.
    pub fn add_adblock_rule(&mut self, rule: &str) {
        self.adblock.borrow_mut().add_rule(rule);
    }

    /// Resets the count of blocked requests.
    #[inline]
    pub fn reset_blocked_count(&self) {
        self.adblock.borrow().reset_blocked_count();
    }

    /// Returns the high-resolution performance metrics from the most recent rendering cycle.
    #[inline]
    pub fn frame_metrics(&self) -> FrameMetrics {
        self.last_metrics
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

    /// Clears the in-memory decoded image cache, freeing bitmap buffers.
    pub fn clear_image_cache(&mut self) {
        self.images.clear();
        self.images.shrink_to_fit();
    }

    /// Releases excess capacity held by internal collections.
    pub fn shrink_to_fit(&mut self) {
        self.images.shrink_to_fit();
        self.stylesheets.shrink_to_fit();
        self.external_stylesheets.shrink_to_fit();
        self.input_buffers.shrink_to_fit();
    }

    /// Returns the current loaded document URL string, if any.
    #[inline]
    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    /// Returns a reference to the loaded DOM tree, if any.
    #[inline]
    pub fn dom(&self) -> Option<&DomTree> {
        self.dom.as_ref()
    }

    /// Returns a reference to the computed layout root box, if any.
    #[inline]
    pub fn layout_root(&self) -> Option<&LayoutBox> {
        self.layout_root.as_ref()
    }

    /// Returns a reference to the computed style tree, if any.
    #[inline]
    pub fn style_tree(&self) -> Option<&StyleTree> {
        self.style_tree.as_ref()
    }

    /// Returns the first `NodeId` whose element matches the given class name.
    pub fn find_node_by_class(&self, class_name: &str) -> Option<NodeId> {
        let dom = self.dom.as_ref()?;
        let mut stack = vec![dom.root_id()];
        while let Some(node_id) = stack.pop() {
            if let Some(node) = dom.get_node(node_id)
                && let NodeData::Element(ref elem) = node.data
                && let Some(classes) = elem.get_attribute("class")
                && classes.split_whitespace().any(|c| c == class_name)
            {
                return Some(node_id);
            }
            for child in dom.children(node_id) {
                stack.push(child);
            }
        }
        None
    }

    /// Checks if a given DOM node is present in the computed layout tree.
    /// Elements with `display: none` are completely omitted.
    pub fn is_node_in_layout(&self, node_id: NodeId) -> bool {
        if let Some(ref root) = self.layout_root {
            find_box_for_node(root, node_id).is_some()
        } else {
            false
        }
    }

    /// Loads and parses an HTML document string, with optional user/author CSS.
    pub fn load_html(&mut self, html: &str, css: Option<&str>) {
        let parse_start = std::time::Instant::now();
        let dom = parse_html(html);
        self.last_metrics.parse_time_ms = parse_start.elapsed().as_secs_f32() * 1000.0;
        self.images.clear();
        self.fetch_and_decode_images(&dom, None);
        self.stylesheets.clear();
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;
        self.current_url = None;
        self.csp_policy = None;
        self.focused_element = None;
        self.input_buffers.clear();

        self.external_stylesheets.clear();
        self.author_css = css.map(parse_css);
        self.rebuild_stylesheets(&dom);

        // Initialize SharedDom and JsRuntime
        let shared_dom = std::rc::Rc::new(std::cell::RefCell::new(dom));
        let url_str = self.current_url.as_deref().unwrap_or("about:blank");
        let origin = if let Ok(u) = Url::parse(url_str) {
            nirvana_net::security::Origin::from_url(&u).as_string()
        } else {
            "null".to_string()
        };

        let mut js = nirvana_js::JsRuntime::with_full_context(
            shared_dom.clone(),
            self.storage_backend.clone(),
            &origin,
            self.cookie_jar.clone(),
            url_str,
        );

        let base_url = self.current_url.clone();
        self.fetch_and_run_scripts(&shared_dom, base_url.as_deref(), &mut js);

        self.dom = Some(shared_dom.borrow().clone());
        self.shared_dom = Some(shared_dom);
        js.clear_mutated();
        self.js_runtime = Some(js);
    }

    /// Navigates to a given URL string.
    /// Fetches document, extracts and fetches linked external stylesheets, and renders.
    pub fn navigate(&mut self, url_str: &str) -> Result<(), String> {
        let url = Url::parse(url_str).map_err(|e| e.to_string())?;

        // 1. Prepare dynamic request headers (e.g. Cookie: ...)
        let cookie_header = self.cookie_jar.borrow().get_cookie_header_value(&url, true);
        let mut req_headers = Vec::new();
        if let Some(ref val) = cookie_header {
            req_headers.push(("Cookie", val.as_str()));
        }

        let resp = self
            .loader
            .fetch_with_headers(&url, &req_headers)
            .map_err(|e| e.to_string())?;

        if resp.status < 200 || resp.status >= 300 {
            return Err(format!("HTTP error status: {}", resp.status));
        }

        // 2. Extract and store Set-Cookie headers from HTTP response
        let default_domain = url.host().unwrap_or("");
        let default_path = url.path();
        for (name, val) in &resp.raw_headers {
            if name.eq_ignore_ascii_case("set-cookie")
                && let Some(cookie) =
                    nirvana_net::cookie::parse_set_cookie(val, default_domain, default_path)
            {
                self.cookie_jar.borrow_mut().store(cookie, &url);
            }
        }

        // 3. Parse Content-Security-Policy header if present
        self.csp_policy = resp
            .headers
            .get("content-security-policy")
            .and_then(|h| nirvana_net::security::CspPolicy::parse(h));

        let html = resp.text().map_err(|e| format!("Invalid UTF-8 document: {e}"))?;
        let parse_start = std::time::Instant::now();
        let dom = parse_html(html);
        self.last_metrics.parse_time_ms = parse_start.elapsed().as_secs_f32() * 1000.0;

        // Discover and load linked external stylesheets: <link rel="stylesheet" href="...">
        let doc_origin = nirvana_net::security::Origin::from_url(&url);
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

            // Mixed content check
            if nirvana_net::security::is_mixed_content(&doc_origin, &css_url) {
                eprintln!("Warning: blocked insecure mixed content stylesheet at {css_url}");
                continue;
            }

            // CSP check
            if let Some(ref csp) = self.csp_policy
                && !csp.is_allowed(nirvana_net::security::CspDirective::StyleSrc, &css_url, &doc_origin)
            {
                eprintln!("Warning: blocked stylesheet by CSP at {css_url}");
                continue;
            }

            // AdBlock check
            if self.adblock.borrow().should_block(
                &css_url,
                self.current_url.as_deref(),
                nirvana_adblock::ContentType(nirvana_adblock::ContentType::STYLESHEET),
            ) {
                eprintln!("AdBlock: blocked ad stylesheet at {css_url}");
                continue;
            }

            let css_cookie = self.cookie_jar.borrow().get_cookie_header_value(&css_url, true);
            let mut css_headers = Vec::new();
            if let Some(ref val) = css_cookie {
                css_headers.push(("Cookie", val.as_str()));
            }

            let css_resp = match self.loader.fetch_with_headers(&css_url, &css_headers) {
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

        self.external_stylesheets = external_sheets;
        self.author_css = None;
        self.rebuild_stylesheets(&dom);
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;
        self.current_url = Some(url_str.to_string());
        self.focused_element = None;
        self.input_buffers.clear();

        let shared_dom = std::rc::Rc::new(std::cell::RefCell::new(dom));
        let origin_str = doc_origin.as_string();
        let url_str = url.as_str();
        let mut js = nirvana_js::JsRuntime::with_full_context(
            shared_dom.clone(),
            self.storage_backend.clone(),
            &origin_str,
            self.cookie_jar.clone(),
            &url_str,
        );

        self.fetch_and_run_scripts(&shared_dom, Some(&url_str), &mut js);

        self.dom = Some(shared_dom.borrow().clone());
        self.shared_dom = Some(shared_dom);
        js.clear_mutated();
        self.js_runtime = Some(js);

        self.render();
        Ok(())
    }

    /// Resizes the viewport and framebuffer canvas.
    /// Re-renders existing document if one is loaded.
    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.clamp(1, 16384);
        let height = height.clamp(1, 16384);
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
        let total_start = std::time::Instant::now();

        let has_mutated = self
            .js_runtime
            .as_ref()
            .map(|js| js.has_mutated())
            .unwrap_or(false);

        if has_mutated {
            if let Some(ref s_dom) = self.shared_dom {
                let fresh_dom = s_dom.borrow().clone();
                self.rebuild_stylesheets(&fresh_dom);
                self.dom = Some(fresh_dom);
            }
            if let Some(ref js) = self.js_runtime {
                js.clear_mutated();
            }
        }

        let Some(ref dom) = self.dom else {
            return;
        };

        let style_start = std::time::Instant::now();
        let sheets: Vec<&StyleSheet> = self.stylesheets.iter().collect();
        let style_tree = compute_styles(dom, &sheets);
        let style_time_ms = style_start.elapsed().as_secs_f32() * 1000.0;

        let layout_start = std::time::Instant::now();
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
        let layout_time_ms = layout_start.elapsed().as_secs_f32() * 1000.0;

        let raster_start = std::time::Instant::now();
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
        let raster_time_ms = raster_start.elapsed().as_secs_f32() * 1000.0;
        let total_frame_time_ms = total_start.elapsed().as_secs_f32() * 1000.0;

        self.last_metrics.style_time_ms = style_time_ms;
        self.last_metrics.layout_time_ms = layout_time_ms;
        self.last_metrics.raster_time_ms = raster_time_ms;
        self.last_metrics.total_frame_time_ms = total_frame_time_ms;

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

            // Mixed content check
            if let Some(base) = base_url {
                let doc_origin = nirvana_net::security::Origin::from_url(base);
                if nirvana_net::security::is_mixed_content(&doc_origin, &img_url) {
                    eprintln!("Warning: blocked insecure mixed content image at {img_url}");
                    continue;
                }
            }

            // CSP check
            if let Some(ref csp) = self.csp_policy {
                let doc_origin = base_url
                    .map(nirvana_net::security::Origin::from_url)
                    .unwrap_or_else(|| nirvana_net::security::Origin::from_url(&img_url));
                if !csp.is_allowed(
                    nirvana_net::security::CspDirective::ImgSrc,
                    &img_url,
                    &doc_origin,
                ) {
                    eprintln!("Warning: blocked image by CSP at {img_url}");
                    continue;
                }
            }

            // AdBlock check
            if self.adblock.borrow().should_block(
                &img_url,
                self.current_url.as_deref(),
                nirvana_adblock::ContentType(nirvana_adblock::ContentType::IMAGE),
            ) {
                eprintln!("AdBlock: blocked ad image at {img_url}");
                continue;
            }

            // Check if canonical URL is already cached
            let canonical = img_url.as_str();
            if let Some(existing) = self.images.get(&canonical) {
                self.images.insert(src.clone(), existing.clone());
                continue;
            }

            let img_cookie = self.cookie_jar.borrow().get_cookie_header_value(&img_url, true);
            let mut img_headers = Vec::new();
            if let Some(ref val) = img_cookie {
                img_headers.push(("Cookie", val.as_str()));
            }

            let resp = match self.loader.fetch_with_headers(&img_url, &img_headers) {
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
        let mut default_prevented = false;
        if let Some(ref result) = hit {
            // Dispatch click event to JS runtime
            let mut dom_mutated = false;
            if let Some(ref mut js) = self.js_runtime {
                if let Ok(allowed) = js.dispatch_event(result.node_id, "click") {
                    default_prevented = !allowed;
                }
                if js.has_mutated() {
                    dom_mutated = true;
                }
            }

            if dom_mutated {
                self.render();
            }

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

        if default_prevented {
            hit.map(|mut h| {
                h.href = None;
                h
            })
        } else {
            hit
        }
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

        if let Some(ref s_dom) = self.shared_dom {
            let _ = s_dom.borrow_mut().set_attribute(node_id, "value", &curr_text);
        }
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

        if let Some(ref s_dom) = self.shared_dom {
            let _ = s_dom.borrow_mut().set_attribute(node_id, "value", &curr_text);
        }
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

    fn rebuild_stylesheets(&mut self, dom: &DomTree) {
        let mut sheets = self.external_stylesheets.clone();
        sheets.extend(discover_inline_stylesheets(dom));
        if let Some(ref auth) = self.author_css {
            sheets.push(auth.clone());
        }
        // AdBlock Cosmetic Filtering: inject element-hiding CSS
        if let Some(cosmetic_css) = self
            .adblock
            .borrow()
            .cosmetic_stylesheet(self.current_url.as_deref())
        {
            sheets.push(parse_css(&cosmetic_css));
        }
        self.stylesheets = sheets;
    }

    fn fetch_and_run_scripts(
        &mut self,
        shared_dom: &std::rc::Rc<std::cell::RefCell<DomTree>>,
        base_url_str: Option<&str>,
        js: &mut nirvana_js::JsRuntime,
    ) {
        // Inject scriptlet defusers (Google Analytics, GTM stubs) at document_start
        if self.adblock.borrow().is_enabled() {
            let defusers = self.adblock.borrow().defuser_script();
            let _ = js.eval(defusers);
        }

        let scripts = {
            let dom = shared_dom.borrow();
            discover_scripts(&dom)
        };
        let base_url = base_url_str.and_then(|u| Url::parse(u).ok());
        for (src_opt, content_opt) in scripts {
            if let Some(src) = src_opt {
                let script_url_opt = if let Some(ref base) = base_url {
                    base.join(&src).ok()
                } else {
                    Url::parse(&src).ok()
                };
                if let Some(script_url) = script_url_opt {
                    if let Some(ref base) = base_url {
                        let doc_origin = nirvana_net::security::Origin::from_url(base);
                        if nirvana_net::security::is_mixed_content(&doc_origin, &script_url) {
                            eprintln!("Warning: blocked insecure mixed content script at {script_url}");
                            continue;
                        }
                    }

                    // CSP check
                    if let Some(ref csp) = self.csp_policy {
                        let doc_origin = base_url
                            .as_ref()
                            .map(nirvana_net::security::Origin::from_url)
                            .unwrap_or_else(|| nirvana_net::security::Origin::from_url(&script_url));
                        if !csp.is_allowed(
                            nirvana_net::security::CspDirective::ScriptSrc,
                            &script_url,
                            &doc_origin,
                        ) {
                            eprintln!("Warning: blocked script by CSP at {script_url}");
                            continue;
                        }
                    }

                    // AdBlock check
                    if self.adblock.borrow().should_block(
                        &script_url,
                        self.current_url.as_deref(),
                        nirvana_adblock::ContentType(nirvana_adblock::ContentType::SCRIPT),
                    ) {
                        eprintln!("AdBlock: blocked ad/tracking script at {script_url}");
                        continue;
                    }

                    let script_cookie = self.cookie_jar.borrow().get_cookie_header_value(&script_url, true);
                    let mut script_headers = Vec::new();
                    if let Some(ref val) = script_cookie {
                        script_headers.push(("Cookie", val.as_str()));
                    }

                    let resp = match self.loader.fetch_with_headers(&script_url, &script_headers) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("Warning: failed to fetch script at {script_url}: {e}");
                            continue;
                        }
                    };
                    if resp.status >= 200
                        && resp.status < 300
                        && let Ok(script_code) = resp.text()
                        && let Err(e) = js.eval(script_code)
                    {
                        eprintln!("JS execution error in external script {script_url}: {e}");
                    }
                }
            } else if let Some(code) = content_opt
                && let Err(e) = js.eval(&code)
            {
                eprintln!("JS execution error in inline script: {e}");
            }
        }
    }

    /// Evaluates a JavaScript string in the active JS runtime context.
    /// If the script mutated the DOM, updates the local DOM snapshot and triggers a re-render.
    pub fn evaluate_js(&mut self, script: &str) -> Result<String, String> {
        let Some(ref mut js) = self.js_runtime else {
            return Err("JsRuntime is not initialized".to_string());
        };
        let res = js.eval(script);
        if js.has_mutated() {
            self.render();
        }
        res
    }

    /// Steps the active JS timers by `delta_ms` milliseconds and fires expired timers.
    /// If timer callbacks mutated the DOM, updates the local DOM snapshot and triggers a re-render.
    pub fn step_timers(&mut self, delta_ms: u64) -> usize {
        let mut executed = 0;
        let mut mutated = false;
        if let Some(ref mut js) = self.js_runtime {
            executed = js.step_timers(delta_ms);
            if js.has_mutated() {
                mutated = true;
            }
        }
        if mutated {
            self.render();
        }
        executed
    }

    /// Returns a reference to the active JS runtime, if available.
    pub fn js_runtime(&self) -> Option<&nirvana_js::JsRuntime> {
        self.js_runtime.as_ref()
    }

    /// Returns a mutable reference to the active JS runtime, if available.
    pub fn js_runtime_mut(&mut self) -> Option<&mut nirvana_js::JsRuntime> {
        self.js_runtime.as_mut()
    }

    /// Returns a shared reference to the sovereign CookieJar.
    pub fn cookie_jar(&self) -> std::rc::Rc<std::cell::RefCell<nirvana_net::cookie::CookieJar>> {
        self.cookie_jar.clone()
    }

    /// Returns a shared reference to the origin-partitioned Web Storage backend.
    pub fn storage_backend(
        &self,
    ) -> std::rc::Rc<std::cell::RefCell<nirvana_js::storage::StorageBackend>> {
        self.storage_backend.clone()
    }

    /// Empties all stored cookies.
    pub fn clear_cookies(&mut self) {
        self.cookie_jar.borrow_mut().clear();
    }

    /// Clears all local and session Web Storage.
    pub fn clear_storage(&mut self) {
        self.storage_backend.borrow_mut().clear();
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

fn discover_scripts(dom: &DomTree) -> Vec<(Option<String>, Option<String>)> {
    let mut scripts = Vec::new();
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && let Some(tag) = dom.tag_name(node_id)
            && tag.eq_ignore_ascii_case("script")
        {
            let src = dom.get_attribute(node_id, "src").map(|s| s.to_string());
            let mut inline = String::new();
            for child in dom.children(node_id) {
                if dom.node_type(child) == Some(NodeType::Text)
                    && let Some(node) = dom.get_node(child)
                    && let NodeData::Text(ref text) = node.data
                {
                    inline.push_str(text);
                }
            }
            let inline_opt = if inline.trim().is_empty() {
                None
            } else {
                Some(inline)
            };
            scripts.push((src, inline_opt));
        }
        for child in dom.children(node_id).into_iter().rev() {
            stack.push(child);
        }
    }
    scripts
}
