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
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const MOBILE_UA: &str = "Mozilla/5.0 (Android 14; Mobile; rv:132.0) Gecko/132.0 Firefox/132.0";
const DESKTOP_UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:132.0) Gecko/20100101 Firefox/132.0";
const DARK_MODE_CSS: &str = r#"
html, body { background-color: #141413 !important; color: #F4F1EA !important; }
div, section, article, main, header, footer, nav, table, tr, td, th, pre, code {
    background-color: #1E1D1B !important;
    color: #F4F1EA !important;
    border-color: #353430 !important;
}
h1, h2, h3, h4, h5, h6, p, span, li, strong, b { color: #F4F1EA !important; }
a { color: #E89578 !important; }
input, textarea, select, button {
    background-color: #272623 !important;
    color: #F4F1EA !important;
    border-color: #46443F !important;
}
"#;

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
    page_title: Option<String>,
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
    custom_ad_rules: Vec<String>,
    pub last_nav_error: Option<String>,
    pub scale_factor: f32,
    pub desktop_mode: bool,
    pub https_only: bool,
    pub do_not_track: bool,
    pub js_enabled: bool,
    pub force_dark_mode: bool,
}

/// Thread-safe snapshot of engine network & policy configuration for lock-free off-thread navigation prefetching.
#[derive(Debug, Clone)]
pub struct NavSnapshot {
    pub loader: ResourceLoader,
    pub desktop_mode: bool,
    pub https_only: bool,
    pub js_enabled: bool,
    pub do_not_track: bool,
    pub adblock_enabled: bool,
    pub custom_ad_rules: Vec<String>,
    pub initial_cookie: Option<String>,
}

/// Completed off-thread document download, decoded subresources, stylesheets, and prefetched scripts.
#[derive(Debug)]
pub struct PreparedNavigation {
    pub final_url: Url,
    pub dom: DomTree,
    pub page_title: Option<String>,
    pub parse_time_ms: f32,
    pub csp_policy: Option<nirvana_net::security::CspPolicy>,
    pub raw_set_cookies: Vec<(Url, String)>,
    pub external_stylesheets: Vec<StyleSheet>,
    pub decoded_images: Vec<(String, String, Arc<nirvana_gfx::image::DecodedImage>)>,
    pub ordered_scripts: Vec<(Option<String>, Option<String>)>,
    pub blocked_delta: u64,
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
            page_title: None,
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
            custom_ad_rules: Vec::new(),
            last_nav_error: None,
            scale_factor: 1.0,
            desktop_mode: false,
            https_only: false,
            do_not_track: true,
            js_enabled: true,
            force_dark_mode: false,
        }
    }

    /// Returns the active scale factor (e.g. mobile device pixel ratio).
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Updates the display scale factor (density ratio).
    pub fn set_scale_factor(&mut self, scale: f32) {
        let s = scale.clamp(0.5, 10.0);
        if (self.scale_factor - s).abs() > 0.001 {
            self.scale_factor = s;
            if self.dom.is_some() {
                self.render();
            }
        }
    }

    /// Toggles Desktop User-Agent mode.
    pub fn set_desktop_mode(&mut self, desktop: bool) {
        self.desktop_mode = desktop;
    }

    /// Toggles automatic HTTPS-Only upgrade on HTTP navigation.
    pub fn set_https_only(&mut self, enabled: bool) {
        self.https_only = enabled;
    }

    /// Toggles JavaScript script execution for newly loaded pages.
    pub fn set_js_enabled(&mut self, enabled: bool) {
        self.js_enabled = enabled;
    }

    /// Toggles Do-Not-Track (`DNT: 1`) HTTP request header.
    pub fn set_do_not_track(&mut self, enabled: bool) {
        self.do_not_track = enabled;
    }

    /// Toggles Forced Dark Web Contents mode and re-renders active document.
    pub fn set_force_dark_mode(&mut self, enabled: bool) {
        if self.force_dark_mode != enabled {
            self.force_dark_mode = enabled;
            if let Some(dom) = self.dom.clone() {
                self.rebuild_stylesheets(&dom);
                self.render();
            }
        }
    }

    /// Returns the `<title>` extracted from the current document, if available.
    #[inline]
    pub fn page_title(&self) -> Option<&str> {
        self.page_title.as_deref()
    }

    /// Finds matching text runs across the laid-out document and scrolls to `match_index`.
    /// Returns the total count of matches found.
    pub fn find_in_page(&mut self, query: &str, match_index: usize) -> usize {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return 0;
        }
        let Some(ref root) = self.layout_root else {
            return 0;
        };
        let mut match_ys = Vec::new();
        collect_matching_run_ys(root, &q, &mut match_ys);
        let count = match_ys.len();
        if count > 0 {
            let idx = match_index % count;
            let target_y = (match_ys[idx] - 32.0).max(0.0);
            self.scroll_to(self.scroll_x, target_y);
        }
        count
    }

    /// Builds a clean Reader Mode HTML view from the active DOM headings & paragraphs.
    pub fn extract_reader_html(&self) -> Option<String> {
        let dom = self.dom.as_ref()?;
        let title = self
            .page_title
            .clone()
            .unwrap_or_else(|| "Reader View".to_string());
        let mut sections = Vec::new();
        collect_reader_blocks(dom, dom.root_id(), &mut sections);
        if sections.is_empty() {
            return None;
        }
        let escaped_title = escape_html(&title);
        let body_html = sections.join("\n");
        Some(format!(
            "<!DOCTYPE html><html><head><title>{escaped_title}</title>\
            <style>\
            body {{ padding: 18px; background-color: #FAF9F5; color: #191918; font-size: 17px; line-height: 1.6; }}\
            h1 {{ font-size: 24px; color: #191918; margin-bottom: 12px; }}\
            h2 {{ font-size: 20px; color: #191918; margin-top: 16px; margin-bottom: 8px; }}\
            p {{ margin-top: 10px; margin-bottom: 10px; }}\
            .reader-badge {{ font-size: 12px; color: #D97757; margin-bottom: 6px; font-weight: bold; }}\
            </style></head><body>\
            <div class=\"reader-badge\">NIRVANA READER VIEW</div>\
            <h1>{escaped_title}</h1>\
            {body_html}\
            </body></html>"
        ))
    }

    /// Returns extracted plain-text snippet of the active document (used for challenge checks & previews).
    pub fn page_text_snippet(&self, max_chars: usize) -> String {
        let Some(ref dom) = self.dom else {
            return String::new();
        };
        let mut buf = String::new();
        collect_visible_text(dom, dom.root_id(), &mut buf, max_chars);
        buf
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
        if !self.custom_ad_rules.iter().any(|r| r == rule) {
            self.custom_ad_rules.push(rule.to_string());
        }
    }

    /// Captures a lock-free network configuration snapshot for off-thread navigation prefetching.
    pub fn create_nav_snapshot(&self, url_str: &str) -> NavSnapshot {
        let initial_cookie = Url::parse(url_str.trim())
            .ok()
            .and_then(|u| self.cookie_jar.borrow().get_cookie_header_value(&u, true));
        NavSnapshot {
            loader: self.loader.clone(),
            desktop_mode: self.desktop_mode,
            https_only: self.https_only,
            js_enabled: self.js_enabled,
            do_not_track: self.do_not_track,
            adblock_enabled: self.adblock.borrow().is_enabled(),
            custom_ad_rules: self.custom_ad_rules.clone(),
            initial_cookie,
        }
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
        self.page_title = discover_title(&dom);
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
        if self.js_enabled {
            self.fetch_and_run_scripts(&shared_dom, base_url.as_deref(), &mut js);
        }

        self.dom = Some(shared_dom.borrow().clone());
        self.shared_dom = Some(shared_dom);
        js.clear_mutated();
        self.js_runtime = Some(js);
    }

    #[cfg(target_os = "android")]
    fn log_android(msg: &str) {
        use std::ffi::CString;
        unsafe extern "C" {
            fn __android_log_print(
                prio: std::os::raw::c_int,
                tag: *const std::os::raw::c_char,
                fmt: *const std::os::raw::c_char,
                ...
            ) -> std::os::raw::c_int;
        }
        let tag = CString::new("NirvanaEngine").unwrap_or_default();
        let fmt = CString::new("%s").unwrap_or_default();
        let message = CString::new(msg).unwrap_or_default();
        unsafe {
            __android_log_print(6, tag.as_ptr(), fmt.as_ptr(), message.as_ptr());
        }
    }

    #[cfg(not(target_os = "android"))]
    fn log_android(msg: &str) {
        eprintln!("[NirvanaEngine] {msg}");
    }

    fn build_request_headers<'a>(&self, cookie_val: Option<&'a str>) -> Vec<(&'a str, &'a str)> {
        let mut h = Vec::new();
        h.push(("User-Agent", if self.desktop_mode { DESKTOP_UA } else { MOBILE_UA }));
        if self.do_not_track {
            h.push(("DNT", "1"));
        }
        if let Some(c) = cookie_val {
            h.push(("Cookie", c));
        }
        h
    }

    /// Navigates to a given URL string, resolving redirects, stylesheets, images, and scripts.
    pub fn navigate(&mut self, url_str: &str) -> Result<(), String> {
        let snap = self.create_nav_snapshot(url_str);
        match fetch_navigation_offline_safe(url_str, &snap) {
            Ok(prepared) => {
                self.apply_prepared_navigation(prepared);
                Ok(())
            }
            Err(e) => {
                self.last_nav_error = Some(e.clone());
                Err(e)
            }
        }
    }

    /// Commits an off-thread `PreparedNavigation` into engine state and renders the page.
    pub fn apply_prepared_navigation(&mut self, nav: PreparedNavigation) {
        self.last_nav_error = None;
        self.last_metrics.parse_time_ms = nav.parse_time_ms;
        self.page_title = nav.page_title;
        self.csp_policy = nav.csp_policy;
        self.adblock.borrow().add_blocked_count(nav.blocked_delta);

        {
            let mut jar = self.cookie_jar.borrow_mut();
            for (cookie_url, raw_val) in &nav.raw_set_cookies {
                let dom_host = cookie_url.host().unwrap_or("");
                let dom_path = cookie_url.path();
                if let Some(cookie) = nirvana_net::cookie::parse_set_cookie(raw_val, dom_host, dom_path) {
                    jar.store(cookie, cookie_url);
                }
            }
        }

        self.images.clear();
        for (src, canonical, arc_img) in nav.decoded_images {
            self.images.insert(src, arc_img.clone());
            self.images.insert(canonical, arc_img);
        }

        self.external_stylesheets = nav.external_stylesheets;
        self.author_css = None;
        self.rebuild_stylesheets(&nav.dom);
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;

        let final_url_str = nav.final_url.as_str().to_string();
        let doc_origin = nirvana_net::security::Origin::from_url(&nav.final_url);
        let origin_str = doc_origin.as_string();
        self.current_url = Some(final_url_str.clone());
        self.focused_element = None;
        self.input_buffers.clear();

        let shared_dom = std::rc::Rc::new(std::cell::RefCell::new(nav.dom));
        let mut js = nirvana_js::JsRuntime::with_full_context(
            shared_dom.clone(),
            self.storage_backend.clone(),
            &origin_str,
            self.cookie_jar.clone(),
            &final_url_str,
        );

        if self.js_enabled {
            if self.adblock.borrow().is_enabled() {
                let defusers = self.adblock.borrow().defuser_script();
                let _ = js.eval(defusers);
            }
            let js_budget_start = std::time::Instant::now();
            for (_src_opt, code_opt) in nav.ordered_scripts {
                if js_budget_start.elapsed().as_millis() > 300 {
                    break;
                }
                if let Some(code) = code_opt {
                    let _ = js.eval(&code);
                }
            }
        }

        self.dom = Some(shared_dom.borrow().clone());
        self.shared_dom = Some(shared_dom);
        js.clear_mutated();
        self.js_runtime = Some(js);

        self.render();
    }

    /// Resizes the viewport and framebuffer canvas.
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
        let logical_w = (self.width as f32 / self.scale_factor).max(10.0);
        let logical_h = (self.height as f32 / self.scale_factor).max(10.0);
        layout_ctx.viewport_width = logical_w;

        for (node_id, src) in discover_images(dom) {
            if let Some(img) = self.images.get(&src) {
                layout_ctx
                    .image_dimensions
                    .insert(node_id, (img.width as f32, img.height as f32));
            }
        }

        let viewport = Size::new(logical_w, logical_h);
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
            canvas.render_display_list_with_scale(
                &list,
                -self.scroll_x,
                -self.scroll_y,
                self.scale_factor,
            );
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
        let mut jobs: Vec<(String, Url, Option<String>)> = Vec::new();
        let mut seen = HashSet::new();

        for (_node_id, src) in discovered {
            if self.images.contains_key(&src) || !seen.insert(src.clone()) {
                continue;
            }
            if src.starts_with("data:image/svg+xml") {
                continue;
            }
            let img_url = match base_url {
                Some(base) => match base.join(&src) {
                    Ok(u) => u,
                    Err(_) => continue,
                },
                None => match Url::parse(&src) {
                    Ok(u) => u,
                    Err(_) => continue,
                },
            };

            if let Some(base) = base_url {
                let doc_origin = nirvana_net::security::Origin::from_url(base);
                if nirvana_net::security::is_mixed_content(&doc_origin, &img_url) {
                    continue;
                }
            }
            if let Some(ref csp) = self.csp_policy {
                let doc_origin = base_url
                    .map(nirvana_net::security::Origin::from_url)
                    .unwrap_or_else(|| nirvana_net::security::Origin::from_url(&img_url));
                if !csp.is_allowed(
                    nirvana_net::security::CspDirective::ImgSrc,
                    &img_url,
                    &doc_origin,
                ) {
                    continue;
                }
            }
            if self.adblock.borrow().should_block(
                &img_url,
                self.current_url.as_deref(),
                nirvana_adblock::ContentType(nirvana_adblock::ContentType::IMAGE),
            ) {
                continue;
            }

            let canonical = img_url.as_str();
            if let Some(existing) = self.images.get(&canonical) {
                self.images.insert(src.clone(), existing.clone());
                continue;
            }

            let img_cookie = self.cookie_jar.borrow().get_cookie_header_value(&img_url, true);
            jobs.push((src, img_url, img_cookie));
            if jobs.len() >= 36 {
                break;
            }
        }

        if jobs.is_empty() {
            return;
        }

        let ua = if self.desktop_mode { DESKTOP_UA } else { MOBILE_UA };
        let dnt = self.do_not_track;
        let loader = self.loader.clone();

        let results: Vec<(String, String, Arc<nirvana_gfx::image::DecodedImage>)> = if jobs.len() == 1 {
            let (src, img_url, cookie) = &jobs[0];
            let headers = self.build_request_headers(cookie.as_deref());
            match loader.fetch_with_headers(img_url, &headers) {
                Ok(resp) if resp.status >= 200 && resp.status < 300 => {
                    match decode_image(&resp.body) {
                        Ok(dec) => vec![(src.clone(), img_url.as_str(), Arc::new(dec))],
                        Err(_) => Vec::new(),
                    }
                }
                _ => Vec::new(),
            }
        } else {
            std::thread::scope(|s| {
                let handles: Vec<_> = jobs
                    .iter()
                    .map(|(src, img_url, cookie)| {
                        let loader_ref = &loader;
                        s.spawn(move || {
                            let mut headers = Vec::new();
                            headers.push(("User-Agent", ua));
                            if dnt {
                                headers.push(("DNT", "1"));
                            }
                            if let Some(c) = cookie.as_deref() {
                                headers.push(("Cookie", c));
                            }
                            match loader_ref.fetch_with_headers(img_url, &headers) {
                                Ok(resp) if resp.status >= 200 && resp.status < 300 => {
                                    decode_image(&resp.body)
                                        .ok()
                                        .map(|dec| (src.clone(), img_url.as_str(), Arc::new(dec)))
                                }
                                _ => None,
                            }
                        })
                    })
                    .collect();

                handles
                    .into_iter()
                    .filter_map(|h| h.join().ok().flatten())
                    .collect()
            })
        };

        for (src, canonical, arc_img) in results {
            self.images.insert(src, arc_img.clone());
            self.images.insert(canonical, arc_img);
        }
    }

    /// Sets the viewport scroll offset to the specified coordinates, clamped to document bounds.
    /// Re-rasterizes the display list cleanly with active DPI `scale_factor`.
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
            self.canvas.render_display_list_with_scale(
                list,
                -self.scroll_x,
                -self.scroll_y,
                self.scale_factor,
            );
        }
    }

    /// Scrolls the viewport by the given logical delta `(dx, dy)`, clamped to document bounds.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.scroll_to(self.scroll_x + dx, self.scroll_y + dy);
    }

    /// Returns current viewport scroll offset `(scroll_x, scroll_y)` in logical units.
    #[inline]
    pub fn scroll_offset(&self) -> (f32, f32) {
        (self.scroll_x, self.scroll_y)
    }

    /// Returns the total scrollable document dimensions `(width, height)`.
    pub fn document_dimensions(&self) -> (f32, f32) {
        if let Some(ref root) = self.layout_root {
            calculate_content_extent(root)
        } else {
            (self.width as f32, self.height as f32)
        }
    }

    /// Returns the maximum scroll offset `(max_x, max_y)` in logical coordinates.
    pub fn max_scroll(&self) -> (f32, f32) {
        let (doc_w, doc_h) = self.document_dimensions();
        let logical_w = self.width as f32 / self.scale_factor;
        let logical_h = self.height as f32 / self.scale_factor;
        let max_x = (doc_w - logical_w).max(0.0);
        let max_y = (doc_h - logical_h).max(0.0);
        (max_x, max_y)
    }

    /// Performs hit-testing at physical screen coordinates `(screen_x, screen_y)`.
    pub fn hit_test(&self, screen_x: f32, screen_y: f32) -> Option<HitTestResult> {
        let (root, dom) = (self.layout_root.as_ref()?, self.dom.as_ref()?);
        let doc_x = (screen_x / self.scale_factor) + self.scroll_x;
        let doc_y = (screen_y / self.scale_factor) + self.scroll_y;
        let hit_node_id = root.hit_test(Point::new(doc_x, doc_y))?;
        Some(resolve_hit_target(dom, hit_node_id))
    }

    /// Handles a click/touch tap at physical screen coordinates `(screen_x, screen_y)`.
    pub fn handle_click(&mut self, screen_x: f32, screen_y: f32) -> Option<HitTestResult> {
        let hit = self.hit_test(screen_x, screen_y);
        let mut default_prevented = false;
        if let Some(ref result) = hit {
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
                let doc_x = (screen_x / self.scale_factor) + self.scroll_x;
                let rel_x = if let Some(ref root) = self.layout_root
                    && let Some(lbox) = find_box_for_node(root, result.node_id)
                {
                    (doc_x - lbox.dimensions.content.origin.x).max(0.0)
                } else {
                    0.0
                };

                let font_size = self
                    .style_tree
                    .as_ref()
                    .and_then(|st| st.get_style(result.node_id))
                    .map(|s| s.font_size)
                    .unwrap_or(16.0);

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
                    let w = font.measure_text(&char_repr, font_size).width;
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

    /// Returns `true` if an `<input>` or `<textarea>` element currently has focus.
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

    /// Returns the physical viewport screen bounds of the currently focused form element.
    pub fn focused_bounds(&self) -> Option<Rect> {
        let node_id = self.focused_element?;
        let root = self.layout_root.as_ref()?;
        let lbox = find_box_for_node(root, node_id)?;
        let border_box = lbox.dimensions.border_box();
        let s = self.scale_factor;
        Some(Rect::new(
            (border_box.x() - self.scroll_x) * s,
            (border_box.y() - self.scroll_y) * s,
            border_box.width() * s,
            border_box.height() * s,
        ))
    }

    /// Inserts `text` into the focused form control at the current caret position.
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

    /// Deletes the character preceding the caret in the focused form control.
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
        if let Some(cosmetic_css) = self
            .adblock
            .borrow()
            .cosmetic_stylesheet(self.current_url.as_deref())
        {
            sheets.push(parse_css(&cosmetic_css));
        }
        if self.force_dark_mode {
            sheets.push(parse_css(DARK_MODE_CSS));
        }
        self.stylesheets = sheets;
    }

    fn fetch_and_run_scripts(
        &mut self,
        shared_dom: &std::rc::Rc<std::cell::RefCell<DomTree>>,
        base_url_str: Option<&str>,
        js: &mut nirvana_js::JsRuntime,
    ) {
        if self.adblock.borrow().is_enabled() {
            let defusers = self.adblock.borrow().defuser_script();
            let _ = js.eval(defusers);
        }

        let scripts = {
            let dom = shared_dom.borrow();
            discover_scripts(&dom)
        };
        let base_url = base_url_str.and_then(|u| Url::parse(u).ok());
        let mut ext_count = 0usize;
        for (src_opt, content_opt) in scripts {
            if let Some(src) = src_opt {
                if ext_count >= 12 {
                    continue;
                }
                let script_url_opt = if let Some(ref base) = base_url {
                    base.join(&src).ok()
                } else {
                    Url::parse(&src).ok()
                };
                if let Some(script_url) = script_url_opt {
                    if let Some(ref base) = base_url {
                        let doc_origin = nirvana_net::security::Origin::from_url(base);
                        if nirvana_net::security::is_mixed_content(&doc_origin, &script_url) {
                            continue;
                        }
                    }

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
                            continue;
                        }
                    }

                    if self.adblock.borrow().should_block(
                        &script_url,
                        self.current_url.as_deref(),
                        nirvana_adblock::ContentType(nirvana_adblock::ContentType::SCRIPT),
                    ) {
                        continue;
                    }

                    ext_count += 1;
                    let script_cookie = self.cookie_jar.borrow().get_cookie_header_value(&script_url, true);
                    let script_headers = self.build_request_headers(script_cookie.as_deref());

                    if let Ok(resp) = self.loader.fetch_with_headers(&script_url, &script_headers)
                        && resp.status >= 200
                        && resp.status < 300
                    {
                        let script_code = resp.decoded_text();
                        let _ = js.eval(&script_code);
                    }
                }
            } else if let Some(code) = content_opt {
                let _ = js.eval(&code);
            }
        }
    }

    /// Evaluates a JavaScript string in the active JS runtime context.
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

    /// Steps the active JS event loop timers by `delta_ms` milliseconds and fires expired timers.
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
    let mut stack = vec![layout_box];
    while let Some(b) = stack.pop() {
        if b.node_id() == Some(target_id) {
            return Some(b);
        }
        for child in b.children.iter().rev() {
            stack.push(child);
        }
    }
    None
}

fn calculate_content_extent(layout_box: &LayoutBox) -> (f32, f32) {
    let mut max_x = 0.0f32;
    let mut max_y = 0.0f32;
    let mut stack = vec![layout_box];
    while let Some(b) = stack.pop() {
        let mb = b.dimensions.margin_box();
        max_x = max_x.max(mb.right());
        max_y = max_y.max(mb.bottom());
        for child in &b.children {
            stack.push(child);
        }
    }
    (max_x, max_y)
}

fn should_skip_subtree(dom: &DomTree, node_id: NodeId) -> bool {
    dom.node_type(node_id) == Some(NodeType::Element)
        && dom.tag_name(node_id).is_some_and(|t| {
            t.eq_ignore_ascii_case("noscript") || t.eq_ignore_ascii_case("template")
        })
}

fn discover_title(dom: &DomTree) -> Option<String> {
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if dom.node_type(node_id) == Some(NodeType::Element)
            && dom.tag_name(node_id).is_some_and(|t| t.eq_ignore_ascii_case("title"))
        {
            let mut text = String::new();
            for child in dom.children(node_id) {
                if let Some(n) = dom.get_node(child)
                    && let NodeData::Text(ref t) = n.data
                {
                    text.push_str(t);
                }
            }
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        for child in dom.children(node_id).into_iter().rev() {
            stack.push(child);
        }
    }
    None
}

fn discover_linked_stylesheets(dom: &DomTree) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut stack = vec![dom.root_id()];
    while let Some(node_id) = stack.pop() {
        if should_skip_subtree(dom, node_id) {
            continue;
        }
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
        if should_skip_subtree(dom, node_id) {
            continue;
        }
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
        if should_skip_subtree(dom, node_id) {
            continue;
        }
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
        if should_skip_subtree(dom, node_id) {
            continue;
        }
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

fn collect_matching_run_ys(layout_box: &LayoutBox, q_lower: &str, out: &mut Vec<f32>) {
    for run in &layout_box.text_runs {
        if run.text.to_ascii_lowercase().contains(q_lower) {
            out.push(run.rect.origin.y);
        }
    }
    for child in &layout_box.children {
        collect_matching_run_ys(child, q_lower, out);
    }
}

fn collect_visible_text(dom: &DomTree, node_id: NodeId, buf: &mut String, max_chars: usize) {
    if buf.len() >= max_chars {
        return;
    }
    if dom.node_type(node_id) == Some(NodeType::Element)
        && dom.tag_name(node_id).is_some_and(|t| {
            matches!(
                t.to_ascii_lowercase().as_str(),
                "script" | "style" | "noscript" | "head" | "svg"
            )
        })
    {
        return;
    }
    if let Some(n) = dom.get_node(node_id)
        && let NodeData::Text(ref t) = n.data
    {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            let starts_punct = trimmed
                .chars()
                .next()
                .is_some_and(|c| matches!(c, ',' | '.' | ';' | ':' | '!' | '?' | ')' | ']' | '%' | '\''));
            if !buf.is_empty() && !starts_punct {
                buf.push(' ');
            }
            buf.push_str(trimmed);
        }
        return;
    }
    for child in dom.children(node_id) {
        collect_visible_text(dom, child, buf, max_chars);
    }
}

fn collect_reader_blocks(dom: &DomTree, node_id: NodeId, out: &mut Vec<String>) {
    if out.len() >= 150 {
        return;
    }
    if dom.node_type(node_id) == Some(NodeType::Element)
        && let Some(tag) = dom.tag_name(node_id)
    {
        let lower = tag.to_ascii_lowercase();
        if matches!(lower.as_str(), "script" | "style" | "noscript" | "nav" | "footer" | "svg" | "aside") {
            return;
        }
        if matches!(lower.as_str(), "p" | "h1" | "h2" | "h3" | "li" | "blockquote") {
            let mut txt = String::new();
            collect_visible_text(dom, node_id, &mut txt, 2000);
            let trimmed = txt.trim();
            if trimmed.len() >= 12 {
                let tag_out = if lower == "h1" { "h2" } else if lower == "li" { "p" } else { lower.as_str() };
                out.push(format!("<{tag_out}>{}</{tag_out}>", escape_html(trimmed)));
            }
            return;
        }
    }
    for child in dom.children(node_id) {
        collect_reader_blocks(dom, child, out);
    }
}

fn escape_html(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '&' => s.push_str("&amp;"),
            '"' => s.push_str("&quot;"),
            _ => s.push(c),
        }
    }
    s
}

/// Executes full HTTP document fetch, redirects, meta-refresh, and concurrent CSS/Image/JS prefetching
/// without holding any lock on `NirvanaEngine`.
pub fn fetch_navigation_offline_safe(
    url_str: &str,
    snap: &NavSnapshot,
) -> Result<PreparedNavigation, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    let mut effective_url_str = url_str.trim().to_string();
    if snap.https_only && effective_url_str.starts_with("http://") {
        let host_part = effective_url_str.trim_start_matches("http://");
        if !host_part.starts_with("localhost") && !host_part.starts_with("127.0.0.1") {
            effective_url_str = format!("https://{host_part}");
        }
    }

    NirvanaEngine::log_android(&format!("navigate: {effective_url_str}"));
    let mut url = Url::parse(&effective_url_str).map_err(|e| format!("Invalid URL: {e}"))?;
    let ua = if snap.desktop_mode { DESKTOP_UA } else { MOBILE_UA };

    let mut req_headers: Vec<(&str, &str)> = vec![("User-Agent", ua)];
    if snap.do_not_track {
        req_headers.push(("DNT", "1"));
    }
    if let Some(ref c) = snap.initial_cookie {
        req_headers.push(("Cookie", c.as_str()));
    }

    let mut resp = snap
        .loader
        .fetch_with_headers(&url, &req_headers)
        .map_err(|e| format!("Network error: {e}"))?;

    if let Some(ref resolved) = resp.final_url
        && let Ok(parsed_final) = Url::parse(resolved)
    {
        url = parsed_final;
    }

    // Manual 3xx redirect backup
    for _ in 0..5 {
        if resp.status >= 300 && resp.status < 400
            && let Some(loc) = resp.headers.get("location").cloned()
            && let Ok(next_url) = url.join(&loc)
        {
            url = next_url;
            let mut next_h: Vec<(&str, &str)> = vec![("User-Agent", ua)];
            if snap.do_not_track {
                next_h.push(("DNT", "1"));
            }
            resp = snap
                .loader
                .fetch_with_headers(&url, &next_h)
                .map_err(|e| format!("Redirect failed to {url}: {e}"))?;
            if let Some(ref resolved) = resp.final_url
                && let Ok(parsed_final) = Url::parse(resolved)
            {
                url = parsed_final;
            }
            continue;
        }
        break;
    }

    let mut raw_set_cookies: Vec<(Url, String)> = Vec::new();
    for (name, val) in &resp.raw_headers {
        if name.eq_ignore_ascii_case("set-cookie") {
            raw_set_cookies.push((url.clone(), val.clone()));
        }
    }

    let mut decoded = resp.decoded_text();

    // Follow valid <meta http-equiv="refresh"> redirects (skip JS-challenge /enablejs gates)
    for _ in 0..2 {
        if let Some(refresh_target) = extract_meta_refresh_url(&decoded)
            && !refresh_target.contains("/enablejs")
            && !refresh_target.contains("httpservice/retry")
            && let Ok(next_url) = url.join(&refresh_target)
            && next_url.as_str() != url.as_str()
        {
            let mut next_h: Vec<(&str, &str)> = vec![("User-Agent", ua)];
            if snap.do_not_track {
                next_h.push(("DNT", "1"));
            }
            if let Ok(next_resp) = snap.loader.fetch_with_headers(&next_url, &next_h) {
                url = next_resp
                    .final_url
                    .as_deref()
                    .and_then(|r| Url::parse(r).ok())
                    .unwrap_or(next_url);
                for (name, val) in &next_resp.raw_headers {
                    if name.eq_ignore_ascii_case("set-cookie") {
                        raw_set_cookies.push((url.clone(), val.clone()));
                    }
                }
                resp = next_resp;
                decoded = resp.decoded_text();
                continue;
            }
        }
        break;
    }

    let csp_policy = resp
        .headers
        .get("content-security-policy")
        .and_then(|h| nirvana_net::security::CspPolicy::parse(h));

    let html: String = if decoded.trim().is_empty() {
        if resp.status >= 400 {
            format!(
                "<!DOCTYPE html><html><body style='padding:24px;'><h2>HTTP {} Error</h2><p>Unable to load {}</p></body></html>",
                resp.status, url
            )
        } else {
            return Err(format!("Empty response body from {}", url));
        }
    } else {
        decoded
    };

    let parse_start = std::time::Instant::now();
    let dom = parse_html(&html);
    let parse_time_ms = parse_start.elapsed().as_secs_f32() * 1000.0;
    let page_title = discover_title(&dom);

    let mut local_adblock = nirvana_adblock::AdBlockEngine::new();
    local_adblock.set_enabled(snap.adblock_enabled);
    for r in &snap.custom_ad_rules {
        local_adblock.add_rule(r);
    }

    let doc_url_str = url.as_str();
    let doc_origin = nirvana_net::security::Origin::from_url(&url);

    enum SubJob {
        Css(usize, Url),
        Image(String, String, Url),
        Script(usize, Url),
    }

    let mut jobs: Vec<SubJob> = Vec::new();
    let mut seen_css = HashSet::new();
    let mut css_count = 0usize;
    for href in discover_linked_stylesheets(&dom) {
        if css_count >= 14 {
            break;
        }
        let Ok(css_url) = url.join(&href) else { continue };
        let canonical = css_url.as_str();
        if !seen_css.insert(canonical) {
            continue;
        }
        if nirvana_net::security::is_mixed_content(&doc_origin, &css_url) {
            continue;
        }
        if let Some(ref csp) = csp_policy
            && !csp.is_allowed(nirvana_net::security::CspDirective::StyleSrc, &css_url, &doc_origin)
        {
            continue;
        }
        if local_adblock.should_block(
            &css_url,
            Some(&doc_url_str),
            nirvana_adblock::ContentType(nirvana_adblock::ContentType::STYLESHEET),
        ) {
            continue;
        }
        jobs.push(SubJob::Css(css_count, css_url));
        css_count += 1;
    }

    let mut seen_img = HashSet::new();
    let mut img_count = 0usize;
    for (_nid, src) in discover_images(&dom) {
        if img_count >= 24 {
            break;
        }
        if !seen_img.insert(src.clone()) || src.starts_with("data:image/svg+xml") {
            continue;
        }
        let Ok(img_url) = url.join(&src) else { continue };
        if nirvana_net::security::is_mixed_content(&doc_origin, &img_url) {
            continue;
        }
        if let Some(ref csp) = csp_policy
            && !csp.is_allowed(nirvana_net::security::CspDirective::ImgSrc, &img_url, &doc_origin)
        {
            continue;
        }
        if local_adblock.should_block(
            &img_url,
            Some(&doc_url_str),
            nirvana_adblock::ContentType(nirvana_adblock::ContentType::IMAGE),
        ) {
            continue;
        }
        let canonical = img_url.as_str();
        jobs.push(SubJob::Image(src, canonical, img_url));
        img_count += 1;
    }

    let mut ordered_scripts: Vec<(Option<String>, Option<String>)> = if snap.js_enabled {
        discover_scripts(&dom)
    } else {
        Vec::new()
    };
    let mut ext_script_count = 0usize;
    if snap.js_enabled {
        for (idx, (src_opt, _)) in ordered_scripts.iter().enumerate() {
            if ext_script_count >= 6 {
                break;
            }
            let Some(ref src) = *src_opt else { continue };
            let Ok(script_url) = url.join(src) else { continue };
            if nirvana_net::security::is_mixed_content(&doc_origin, &script_url) {
                continue;
            }
            if let Some(ref csp) = csp_policy
                && !csp.is_allowed(nirvana_net::security::CspDirective::ScriptSrc, &script_url, &doc_origin)
            {
                continue;
            }
            if local_adblock.should_block(
                &script_url,
                Some(&doc_url_str),
                nirvana_adblock::ContentType(nirvana_adblock::ContentType::SCRIPT),
            ) {
                continue;
            }
            jobs.push(SubJob::Script(idx, script_url));
            ext_script_count += 1;
        }
    }

    let css_results: Mutex<Vec<(usize, StyleSheet)>> = Mutex::new(Vec::new());
    let img_results: Mutex<Vec<(String, String, Arc<nirvana_gfx::image::DecodedImage>)>> =
        Mutex::new(Vec::new());
    let script_results: Mutex<Vec<(usize, String)>> = Mutex::new(Vec::new());

    if !jobs.is_empty() {
        let next_idx = AtomicUsize::new(0);
        let budget_start = std::time::Instant::now();
        let workers = jobs.len().min(10);
        let loader_ref = &snap.loader;
        let dnt = snap.do_not_track;

        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| {
                    let mut headers = Vec::new();
                    headers.push(("User-Agent", ua));
                    if dnt {
                        headers.push(("DNT", "1"));
                    }
                    loop {
                        if budget_start.elapsed().as_secs_f32() > 7.5 {
                            break;
                        }
                        let i = next_idx.fetch_add(1, Ordering::Relaxed);
                        let Some(job) = jobs.get(i) else { break };
                        match job {
                            SubJob::Css(order_idx, css_url) => {
                                if let Ok(r) = loader_ref.fetch_fast_with_headers(css_url, &headers)
                                    && r.status >= 200
                                    && r.status < 300
                                    && r.body.len() <= 1_048_576
                                {
                                    let sheet = parse_css(&r.decoded_text());
                                    css_results.lock().unwrap().push((*order_idx, sheet));
                                }
                            }
                            SubJob::Image(src, canonical, img_url) => {
                                if let Ok(r) = loader_ref.fetch_fast_with_headers(img_url, &headers)
                                    && r.status >= 200
                                    && r.status < 300
                                    && r.body.len() <= 4_194_304
                                    && let Ok(dec) = decode_image(&r.body)
                                {
                                    img_results.lock().unwrap().push((
                                        src.clone(),
                                        canonical.clone(),
                                        Arc::new(dec),
                                    ));
                                }
                            }
                            SubJob::Script(dom_idx, script_url) => {
                                if let Ok(r) = loader_ref.fetch_fast_with_headers(script_url, &headers)
                                    && r.status >= 200
                                    && r.status < 300
                                    && r.body.len() <= 524_288
                                {
                                    script_results.lock().unwrap().push((*dom_idx, r.decoded_text()));
                                }
                            }
                        }
                    }
                });
            }
        });
    }

    let mut css_vec = css_results.into_inner().unwrap();
    css_vec.sort_by_key(|(idx, _)| *idx);
    let external_stylesheets: Vec<StyleSheet> = css_vec.into_iter().map(|(_, s)| s).collect();

    for (idx, code) in script_results.into_inner().unwrap() {
        if let Some(slot) = ordered_scripts.get_mut(idx) {
            slot.1 = Some(code);
        }
    }

    Ok(PreparedNavigation {
        final_url: url,
        dom,
        page_title,
        parse_time_ms,
        csp_policy,
        raw_set_cookies,
        external_stylesheets,
        decoded_images: img_results.into_inner().unwrap(),
        ordered_scripts,
        blocked_delta: local_adblock.blocked_count(),
    })
}

fn extract_meta_refresh_url(html: &str) -> Option<String> {
    let head_slice = &html[..html.len().min(8192)];
    let lower = head_slice.to_ascii_lowercase();
    let mut search_pos = 0usize;
    while let Some(rel) = lower[search_pos..].find("<meta") {
        let start = search_pos + rel;
        let Some(end_rel) = lower[start..].find('>') else { break };
        let tag_raw = &head_slice[start..start + end_rel + 1];
        let tag_low = &lower[start..start + end_rel + 1];
        if tag_low.contains("refresh")
            && let Some(url_idx) = tag_low.find("url=")
        {
            let after = tag_raw[url_idx + 4..].trim_start_matches(['"', '\'', ' ']);
            let end_url = after
                .find(|c: char| matches!(c, '"' | '\'' | '>' | ' ' | '\r' | '\n'))
                .unwrap_or(after.len());
            let target = after[..end_url].trim().replace("&amp;", "&");
            if !target.is_empty() {
                return Some(target);
            }
        }
        search_pos = start + end_rel + 1;
    }
    None
}
