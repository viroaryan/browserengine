//! Core browser engine state machine for Android embedding.

use nirvana_core::geometry::Size;
use nirvana_css::ast::StyleSheet;
use nirvana_css::parser::parse_css;
use nirvana_dom::DomTree;
use nirvana_gfx::builder::build_display_list;
use nirvana_gfx::canvas::Canvas;
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
        self.canvas = Canvas::new(width, height);

        if self.dom.is_some() {
            self.render();
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
        if let Some(ref root) = layout_root {
            let display_list = build_display_list(root, &style_tree, dom);
            canvas.render_display_list(&display_list);
        }

        self.style_tree = Some(style_tree);
        self.layout_root = layout_root;
        self.canvas = canvas;
    }
}
