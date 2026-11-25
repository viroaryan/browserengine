//! Style cascade, indexed selector lookup, inheritance resolution, and computed style tree generation.

use crate::matcher::matches_selector;
use crate::user_agent::user_agent_stylesheet;
use crate::values::{ComputedValues, Display, Length, Position, TextAlign};
use nirvana_core::arena::Id;
use nirvana_core::color::Rgba;
use nirvana_css::ast::{Declaration, PropertyValue, Selector, SimpleSelector, Specificity, StyleSheet};
use nirvana_dom::node::Node;
use nirvana_dom::{DomTree, NodeType};
use std::collections::HashMap;

const ROOT_FONT_SIZE_PX: f32 = 16.0;
const ASSUMED_VW_PX: f32 = 3.92;
const ASSUMED_VH_PX: f32 = 8.0;

/// Maps DOM elements to their fully computed CSS styles.
#[derive(Debug, Clone, Default)]
pub struct StyleTree {
    styles: HashMap<Id<Node>, ComputedValues>,
}

impl StyleTree {
    /// Returns the computed style for a given DOM node.
    pub fn get_style(&self, id: Id<Node>) -> Option<&ComputedValues> {
        self.styles.get(&id)
    }

    /// Inserts or replaces the computed style for a node.
    pub fn insert(&mut self, id: Id<Node>, values: ComputedValues) {
        self.styles.insert(id, values);
    }
}

struct MatchedDecl<'a> {
    important: bool,
    origin: u8,
    specificity: Specificity,
    source_order: usize,
    decl: &'a Declaration,
}

#[derive(Clone, Copy)]
struct IndexedSelector<'a> {
    origin: u8,
    source_order: usize,
    selector: &'a Selector,
    declarations: &'a [Declaration],
}

/// Subject-keyed CSS rule index enabling O(matching buckets) cascade evaluation instead of O(rules * nodes).
struct RuleIndex<'a> {
    by_id: HashMap<&'a str, Vec<IndexedSelector<'a>>>,
    by_class: HashMap<&'a str, Vec<IndexedSelector<'a>>>,
    by_tag: HashMap<String, Vec<IndexedSelector<'a>>>,
    universal: Vec<IndexedSelector<'a>>,
}

impl<'a> RuleIndex<'a> {
    fn build(ua_sheet: &'a StyleSheet, author_sheets: &[&'a StyleSheet]) -> Self {
        let mut idx = Self {
            by_id: HashMap::new(),
            by_class: HashMap::new(),
            by_tag: HashMap::new(),
            universal: Vec::new(),
        };
        let mut order = 0usize;

        for rule in &ua_sheet.rules {
            for sel in &rule.selectors {
                idx.insert(IndexedSelector {
                    origin: 0,
                    source_order: order,
                    selector: sel,
                    declarations: &rule.declarations,
                });
            }
            order += 1;
        }

        for sheet in author_sheets {
            for rule in &sheet.rules {
                for sel in &rule.selectors {
                    idx.insert(IndexedSelector {
                        origin: 1,
                        source_order: order,
                        selector: sel,
                        declarations: &rule.declarations,
                    });
                }
                order += 1;
            }
        }
        idx
    }

    fn insert(&mut self, entry: IndexedSelector<'a>) {
        let subject = &entry.selector.simple_selectors;
        if let Some(id_str) = subject.iter().find_map(|s| match s {
            SimpleSelector::Id(id) => Some(id.as_str()),
            _ => None,
        }) {
            self.by_id.entry(id_str).or_default().push(entry);
            return;
        }
        if let Some(cls_str) = subject.iter().find_map(|s| match s {
            SimpleSelector::Class(c) => Some(c.as_str()),
            _ => None,
        }) {
            self.by_class.entry(cls_str).or_default().push(entry);
            return;
        }
        if let Some(tag_str) = subject.iter().find_map(|s| match s {
            SimpleSelector::Tag(t) => Some(t.to_ascii_lowercase()),
            _ => None,
        }) {
            self.by_tag.entry(tag_str).or_default().push(entry);
            return;
        }
        self.universal.push(entry);
    }

    fn collect_candidates(&self, dom: &DomTree, node_id: Id<Node>, out: &mut Vec<IndexedSelector<'a>>) {
        out.clear();
        out.extend_from_slice(&self.universal);

        if let Some(tag) = dom.tag_name(node_id) {
            let lower = tag.to_ascii_lowercase();
            if let Some(entries) = self.by_tag.get(lower.as_str()) {
                out.extend_from_slice(entries);
            }
        }
        if let Some(id_attr) = dom.get_attribute(node_id, "id")
            && let Some(entries) = self.by_id.get(id_attr)
        {
            out.extend_from_slice(entries);
        }
        if let Some(cls_attr) = dom.get_attribute(node_id, "class") {
            let mut seen_cls: Vec<&str> = Vec::new();
            for cls in cls_attr.split_ascii_whitespace() {
                if !seen_cls.contains(&cls) {
                    seen_cls.push(cls);
                    if let Some(entries) = self.by_class.get(cls) {
                        out.extend_from_slice(entries);
                    }
                }
            }
        }
    }
}

/// Computes the style tree for a DOM tree against User-Agent and author stylesheets.
pub fn compute_styles(dom: &DomTree, stylesheets: &[&StyleSheet]) -> StyleTree {
    let mut style_tree = StyleTree::default();
    let ua_sheet = user_agent_stylesheet();
    let index = RuleIndex::build(ua_sheet, stylesheets);
    let mut candidates = Vec::with_capacity(64);

    let root = dom.root_id();
    let mut stack: Vec<(Id<Node>, Option<Id<Node>>)> = vec![(root, None)];

    while let Some((node_id, parent_elem_id)) = stack.pop() {
        let is_elem = dom.node_type(node_id) == Some(NodeType::Element);
        if is_elem {
            let parent_style = parent_elem_id.and_then(|pid| style_tree.get_style(pid));
            let computed = compute_single_element_style(
                dom,
                node_id,
                parent_style,
                &index,
                &mut candidates,
            );
            style_tree.insert(node_id, computed);
        }

        let next_parent_elem = if is_elem { Some(node_id) } else { parent_elem_id };
        for child in dom.children(node_id).into_iter().rev() {
            stack.push((child, next_parent_elem));
        }
    }

    style_tree
}

fn compute_single_element_style<'a>(
    dom: &DomTree,
    node_id: Id<Node>,
    parent_style: Option<&ComputedValues>,
    index: &RuleIndex<'a>,
    candidates: &mut Vec<IndexedSelector<'a>>,
) -> ComputedValues {
    let mut computed = ComputedValues::default();

    let parent_font_size = if let Some(parent) = parent_style {
        computed.color = parent.color;
        computed.font_size = parent.font_size;
        computed.font_weight = parent.font_weight;
        computed.font_style_italic = parent.font_style_italic;
        computed.text_decoration_underline = parent.text_decoration_underline;
        computed.text_align = parent.text_align;
        computed.visibility_hidden = parent.visibility_hidden;
        parent.font_size
    } else {
        ROOT_FONT_SIZE_PX
    };

    index.collect_candidates(dom, node_id, candidates);
    let mut matched: Vec<MatchedDecl> = Vec::new();

    for entry in candidates.iter() {
        if matches_selector(node_id, entry.selector, dom) {
            let spec = entry.selector.specificity();
            for decl in entry.declarations {
                matched.push(MatchedDecl {
                    important: decl.important,
                    origin: entry.origin,
                    specificity: spec,
                    source_order: entry.source_order,
                    decl,
                });
            }
        }
    }

    matched.sort_by(|a, b| {
        a.important
            .cmp(&b.important)
            .then_with(|| a.origin.cmp(&b.origin))
            .then_with(|| a.specificity.cmp(&b.specificity))
            .then_with(|| a.source_order.cmp(&b.source_order))
    });

    let (normal, important): (Vec<_>, Vec<_>) = matched.into_iter().partition(|d| !d.important);

    for item in normal {
        apply_declaration(&mut computed, item.decl, parent_font_size);
    }

    let mut inline_important = Vec::new();
    if let Some(style_attr) = dom.get_attribute(node_id, "style") {
        let inline_decls = nirvana_css::parse_inline_declarations(style_attr);
        for decl in inline_decls {
            if decl.important {
                inline_important.push(decl);
            } else {
                apply_declaration(&mut computed, &decl, parent_font_size);
            }
        }
    }

    for item in important {
        apply_declaration(&mut computed, item.decl, parent_font_size);
    }
    for decl in &inline_important {
        apply_declaration(&mut computed, decl, parent_font_size);
    }

    apply_html_presentational_attrs(dom, node_id, &mut computed);

    // Hide CSS visually-hidden / .sr-only / 1px skip-link elements cleanly
    if let (Length::Px(w), Length::Px(h)) = (computed.width, computed.height)
        && w <= 2.0
        && h <= 2.0
        && (computed.position != Position::Static || computed.overflow_hidden)
    {
        computed.offscreen_hidden = true;
    }

    computed
}

fn apply_html_presentational_attrs(
    dom: &DomTree,
    node_id: Id<Node>,
    computed: &mut ComputedValues,
) {
    if dom.get_attribute(node_id, "hidden").is_some() {
        computed.display = Display::None;
    }
    if let Some(align) = dom.get_attribute(node_id, "align") {
        match align.to_ascii_lowercase().as_str() {
            "center" => computed.text_align = TextAlign::Center,
            "right" => computed.text_align = TextAlign::Right,
            "left" => computed.text_align = TextAlign::Left,
            _ => {}
        }
    }
    if computed.background_color.a == 0
        && let Some(bg) = dom.get_attribute(node_id, "bgcolor")
        && let Ok(c) = Rgba::from_hex(bg.trim())
    {
        computed.background_color = c;
    }
}

fn apply_declaration(computed: &mut ComputedValues, decl: &Declaration, parent_font_size: f32) {
    let elem_fs = computed.font_size;
    match decl.name.as_str() {
        "display" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                match k.to_ascii_lowercase().as_str() {
                    "block" | "list-item" => {
                        computed.display = if k.eq_ignore_ascii_case("list-item") {
                            Display::ListItem
                        } else {
                            Display::Block
                        };
                        computed.is_grid = false;
                    }
                    "inline" => {
                        computed.display = Display::Inline;
                        computed.is_grid = false;
                    }
                    "inline-block" | "inline-flex" => {
                        computed.display = Display::InlineBlock;
                        computed.is_grid = false;
                    }
                    "flex" => {
                        computed.display = Display::Flex;
                        computed.is_grid = false;
                    }
                    "grid" => {
                        computed.display = Display::Flex;
                        computed.is_grid = true;
                    }
                    "table" => {
                        computed.display = Display::Table;
                        computed.is_grid = false;
                    }
                    "table-row-group" | "table-header-group" | "table-footer-group" => {
                        computed.display = Display::TableRowGroup;
                        computed.is_grid = false;
                    }
                    "table-row" => {
                        computed.display = Display::TableRow;
                        computed.is_grid = false;
                    }
                    "table-cell" => {
                        computed.display = Display::TableCell;
                        computed.is_grid = false;
                    }
                    "none" => computed.display = Display::None,
                    _ => {}
                }
            }
        }
        "position" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                match k.to_ascii_lowercase().as_str() {
                    "static" => computed.position = Position::Static,
                    "relative" => computed.position = Position::Relative,
                    "absolute" => computed.position = Position::Absolute,
                    "fixed" => computed.position = Position::Fixed,
                    "sticky" => computed.position = Position::Sticky,
                    _ => {}
                }
            }
        }
        "transform" => {
            // Detect Tailwind / UI offscreen slide drawers (-translate-x-full / translateX(-100%) / translateX(calc(... * -100%)))
            if let PropertyValue::Keyword(k) = &decl.value {
                let low = k.to_ascii_lowercase();
                if low == "none" {
                    computed.offscreen_hidden = false;
                } else if low.contains("-100%") || low.contains("-999") || low.contains("-100vw") {
                    computed.offscreen_hidden = true;
                }
            } else if let PropertyValue::Percent(pct) = decl.value
                && pct <= -95.0
            {
                computed.offscreen_hidden = true;
            } else if let PropertyValue::Px(px) = decl.value
                && px <= -500.0
            {
                computed.offscreen_hidden = true;
            }
        }
        "clip" | "clip-path" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                let low = k.to_ascii_lowercase();
                if low == "auto" || low == "none" {
                    computed.offscreen_hidden = false;
                } else if low.contains("rect(0") || low.contains("rect(1px") || low.contains("inset(50%") || low.contains("inset(100%") {
                    computed.offscreen_hidden = true;
                }
            }
        }
        "overflow" | "overflow-x" | "overflow-y" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                let low = k.to_ascii_lowercase();
                computed.overflow_hidden = low == "hidden" || low == "clip";
            }
        }
        "left" | "top" | "margin-left" => {
            if decl.name == "margin-left" {
                apply_length(&mut computed.margin_left, &decl.value, elem_fs);
            }
            if let PropertyValue::Px(px) = decl.value
                && px <= -1000.0
            {
                computed.offscreen_hidden = true;
            } else if decl.name != "margin-left"
                && let PropertyValue::Percent(pct) = decl.value
                && pct <= -95.0
            {
                computed.offscreen_hidden = true;
            }
        }
        "color" => {
            if let PropertyValue::Color(c) = &decl.value {
                computed.color = *c;
            }
        }
        "background-color" => {
            if let PropertyValue::Color(c) = &decl.value {
                computed.background_color = *c;
            }
        }
        "border-color" => {
            if let PropertyValue::Color(c) = &decl.value {
                computed.border_color = Some(*c);
            }
        }
        "margin-top" => apply_length(&mut computed.margin_top, &decl.value, elem_fs),
        "margin-right" => apply_length(&mut computed.margin_right, &decl.value, elem_fs),
        "margin-bottom" => apply_length(&mut computed.margin_bottom, &decl.value, elem_fs),
        "padding-top" => apply_padding(&mut computed.padding_top, &decl.value, elem_fs),
        "padding-right" => apply_padding(&mut computed.padding_right, &decl.value, elem_fs),
        "padding-bottom" => apply_padding(&mut computed.padding_bottom, &decl.value, elem_fs),
        "padding-left" => apply_padding(&mut computed.padding_left, &decl.value, elem_fs),
        "border" | "border-width" => {
            if let Some(p) = resolve_px(&decl.value, elem_fs) {
                let p = p.max(0.0);
                computed.border_width = p;
                computed.border_top_width = p;
                computed.border_right_width = p;
                computed.border_bottom_width = p;
                computed.border_left_width = p;
            }
        }
        "border-top-width" => {
            if let Some(p) = resolve_px(&decl.value, elem_fs) {
                let p = p.max(0.0);
                computed.border_top_width = p;
                computed.border_width = computed.border_width.max(p);
            }
        }
        "border-right-width" => {
            if let Some(p) = resolve_px(&decl.value, elem_fs) {
                let p = p.max(0.0);
                computed.border_right_width = p;
                computed.border_width = computed.border_width.max(p);
            }
        }
        "border-bottom-width" => {
            if let Some(p) = resolve_px(&decl.value, elem_fs) {
                let p = p.max(0.0);
                computed.border_bottom_width = p;
                computed.border_width = computed.border_width.max(p);
            }
        }
        "border-left-width" => {
            if let Some(p) = resolve_px(&decl.value, elem_fs) {
                let p = p.max(0.0);
                computed.border_left_width = p;
                computed.border_width = computed.border_width.max(p);
            }
        }
        "border-radius" => {
            if let Some(p) = resolve_px(&decl.value, elem_fs) {
                computed.border_radius = p.max(0.0);
            }
        }
        "width" => apply_length(&mut computed.width, &decl.value, elem_fs),
        "height" => apply_length(&mut computed.height, &decl.value, elem_fs),
        "min-width" => apply_length(&mut computed.min_width, &decl.value, elem_fs),
        "max-width" => apply_length(&mut computed.max_width, &decl.value, elem_fs),
        "min-height" => apply_length(&mut computed.min_height, &decl.value, elem_fs),
        "font-size" => match &decl.value {
            PropertyValue::Percent(pct) if pct.is_finite() => {
                computed.font_size = (parent_font_size * (pct / 100.0)).max(6.0).min(96.0);
            }
            PropertyValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "xx-small" => computed.font_size = 9.0,
                "x-small" => computed.font_size = 10.0,
                "small" => computed.font_size = 13.0,
                "medium" => computed.font_size = 16.0,
                "large" => computed.font_size = 18.0,
                "x-large" => computed.font_size = 24.0,
                "xx-large" => computed.font_size = 32.0,
                "smaller" => computed.font_size = (parent_font_size * 0.85).max(6.0),
                "larger" => computed.font_size = (parent_font_size * 1.2).min(96.0),
                _ => {}
            },
            other => {
                if let Some(px) = resolve_px(other, parent_font_size) {
                    computed.font_size = px.clamp(6.0, 96.0);
                }
            }
        },
        "font-weight" => match &decl.value {
            PropertyValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "bold" | "bolder" => computed.font_weight = 700,
                "normal" | "lighter" => computed.font_weight = 400,
                _ => {}
            },
            PropertyValue::Number(n) if n.is_finite() => {
                computed.font_weight = n.clamp(100.0, 900.0) as u16;
            }
            _ => {}
        },
        "font-style" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                match k.to_ascii_lowercase().as_str() {
                    "italic" | "oblique" => computed.font_style_italic = true,
                    "normal" => computed.font_style_italic = false,
                    _ => {}
                }
            }
        }
        "text-decoration" | "text-decoration-line" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                match k.to_ascii_lowercase().as_str() {
                    "underline" => computed.text_decoration_underline = true,
                    "none" => computed.text_decoration_underline = false,
                    _ => {}
                }
            }
        }
        "text-align" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                match k.to_ascii_lowercase().as_str() {
                    "center" => computed.text_align = TextAlign::Center,
                    "right" | "end" => computed.text_align = TextAlign::Right,
                    "left" | "start" => computed.text_align = TextAlign::Left,
                    _ => {}
                }
            }
        }
        "line-height" => match &decl.value {
            PropertyValue::Number(n) if n.is_finite() => {
                computed.line_height = (*n * elem_fs).max(0.0);
            }
            other => {
                if let Some(px) = resolve_px(other, elem_fs) {
                    computed.line_height = px.max(0.0);
                }
            }
        },
        "flex-direction" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                computed.flex_direction_column = k.to_ascii_lowercase().starts_with("column");
            }
        }
        "flex-wrap" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                computed.flex_wrap = !k.eq_ignore_ascii_case("nowrap");
            }
        }
        "grid-template-columns" => {
            if let PropertyValue::Keyword(k) = &decl.value
                && let Some(num_str) = k.strip_prefix("cols:")
                && let Ok(n) = num_str.parse::<usize>()
            {
                computed.grid_cols = n.clamp(1, 6);
            }
        }
        "gap" | "column-gap" | "row-gap" => {
            if let Some(px) = resolve_px(&decl.value, elem_fs) {
                computed.gap = px.max(0.0);
            }
        }
        "visibility" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                computed.visibility_hidden = k.eq_ignore_ascii_case("hidden") || k.eq_ignore_ascii_case("collapse");
            }
        }
        "opacity" => {
            if let PropertyValue::Number(n) = decl.value
                && n <= 0.0
            {
                computed.visibility_hidden = true;
            }
        }
        _ => {}
    }
}

fn resolve_px(val: &PropertyValue, ref_font_size: f32) -> Option<f32> {
    let raw = match val {
        PropertyValue::Px(p) => Some(*p),
        PropertyValue::Em(e) => Some(*e * ref_font_size),
        PropertyValue::Rem(r) => Some(*r * ROOT_FONT_SIZE_PX),
        PropertyValue::Pt(pt) => Some(*pt * (4.0 / 3.0)),
        PropertyValue::Vw(vw) => Some(*vw * ASSUMED_VW_PX),
        PropertyValue::Vh(vh) => Some(*vh * ASSUMED_VH_PX),
        _ => None,
    };
    raw.filter(|v| v.is_finite())
}

fn apply_padding(target: &mut Length, value: &PropertyValue, font_size: f32) {
    if let PropertyValue::Percent(pct) = value
        && pct.is_finite()
    {
        *target = Length::Percent(*pct);
    } else if let Some(p) = resolve_px(value, font_size) {
        *target = Length::Px(p.max(0.0));
    }
}

fn apply_length(target: &mut Length, value: &PropertyValue, font_size: f32) {
    match value {
        PropertyValue::Auto => *target = Length::Auto,
        PropertyValue::Percent(pct) if pct.is_finite() => *target = Length::Percent(*pct),
        other => {
            if let Some(p) = resolve_px(other, font_size) {
                *target = Length::Px(p);
            }
        }
    }
}
