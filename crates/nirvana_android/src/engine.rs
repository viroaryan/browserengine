//! Core browser engine state machine for Android embedding.

use nirvana_core::geometry::Size;
use nirvana_css::ast::StyleSheet;
use nirvana_css::parser::parse_css;
use nirvana_dom::DomTree;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;
use nirvana_gfx::display_list::DisplayList;
use nirvana_html::parse_html;
use nirvana_layout::box_type::LayoutBox;
use nirvana_layout::layout::layout_document;
use nirvana_style::compute::{compute_styles, StyleTree};

/// Orchestrates HTML parsing, CSS style computation, layout geometry,
/// and pixel rasterization into a 32-bit RGBA framebuffer.
#[derive(Debug, Clone)]
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

    /// Loads and parses an HTML document string, with optional user/author CSS.
    pub fn load_html(&mut self, html: &str, css: Option<&str>) {
        self.dom = Some(parse_html(html));
        self.stylesheets.clear();
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.display_list = None;

        if let Some(css_text) = css {
            let sheet = parse_css(css_text);
            self.stylesheets.push(sheet);
        }
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
