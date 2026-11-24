//! Core browser engine state machine for Android embedding.

use crate::hit_test::{resolve_hit_target, HitTestResult};
use nirvana_core::geometry::{Point, Size};
use nirvana_css::ast::StyleSheet;
use nirvana_css::parser::parse_css;
use nirvana_dom::{DomTree, NodeType};
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;
use nirvana_gfx::display_list::DisplayList;
use nirvana_html::parse_html;
use nirvana_layout::box_type::LayoutBox;
use nirvana_layout::layout::layout_document;
use nirvana_net::loader::ResourceLoader;
use nirvana_style::compute::{compute_styles, StyleTree};
use nirvana_url::Url;

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
        self.dom = Some(parse_html(html));
        self.stylesheets.clear();
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;
        self.current_url = None;

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

        self.dom = Some(dom);
        self.stylesheets = external_sheets;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;
        self.current_url = Some(url_str.to_string());

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

        let viewport = Size::new(self.width as f32, self.height as f32);
        let layout_root = layout_document(dom, &style_tree, viewport);

        let mut canvas = Canvas::new(self.width, self.height);
        let display_list = if let Some(ref root) = layout_root {
            let list = build_display_list(root, &style_tree, dom);
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
