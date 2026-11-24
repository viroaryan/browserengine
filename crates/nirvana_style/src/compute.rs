//! Style cascade, inheritance resolution, and computed style tree generation.

use crate::matcher::matches_selector;
use crate::user_agent::user_agent_stylesheet;
use crate::values::{ComputedValues, Display, Length};
use nirvana_core::arena::Id;
use nirvana_css::ast::{Declaration, PropertyValue, Specificity, StyleSheet};
use nirvana_dom::node::Node;
use nirvana_dom::{DomTree, NodeType};
use std::collections::HashMap;

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

struct MatchedRule<'a> {
    origin: u8,
    specificity: Specificity,
    source_order: usize,
    declarations: &'a [Declaration],
}

/// Computes the style tree for a DOM tree against User-Agent and author stylesheets.
pub fn compute_styles(dom: &DomTree, stylesheets: &[&StyleSheet]) -> StyleTree {
    let mut style_tree = StyleTree::default();
    let root = dom.root_id();
    compute_node_styles(dom, root, None, stylesheets, &mut style_tree);
    style_tree
}

fn compute_node_styles(
    dom: &DomTree,
    node_id: Id<Node>,
    parent_style: Option<&ComputedValues>,
    author_sheets: &[&StyleSheet],
    style_tree: &mut StyleTree,
) {
    if dom.node_type(node_id) == Some(NodeType::Element) {
        let mut computed = ComputedValues::default();

        // Property inheritance: inherit text color from parent if present
        if let Some(parent) = parent_style {
            computed.color = parent.color;
        }

        let mut matched: Vec<MatchedRule> = Vec::new();
        let mut order = 0;

        // 1. User-Agent stylesheet rules (Origin: 0)
        let ua_sheet = user_agent_stylesheet();
        for rule in &ua_sheet.rules {
            for sel in &rule.selectors {
                if matches_selector(node_id, sel, dom) {
                    matched.push(MatchedRule {
                        origin: 0,
                        specificity: sel.specificity(),
                        source_order: order,
                        declarations: &rule.declarations,
                    });
                }
            }
            order += 1;
        }

        // 2. Author stylesheet rules (Origin: 1)
        for sheet in author_sheets {
            for rule in &sheet.rules {
                for sel in &rule.selectors {
                    if matches_selector(node_id, sel, dom) {
                        matched.push(MatchedRule {
                            origin: 1,
                            specificity: sel.specificity(),
                            source_order: order,
                            declarations: &rule.declarations,
                        });
                    }
                }
                order += 1;
            }
        }

        // Sort rules by (origin ASC, specificity ASC, source_order ASC)
        matched.sort_by(|a, b| {
            a.origin
                .cmp(&b.origin)
                .then_with(|| a.specificity.cmp(&b.specificity))
                .then_with(|| a.source_order.cmp(&b.source_order))
        });

        // Apply sorted rule declarations
        for rule in &matched {
            for decl in rule.declarations {
                apply_declaration(&mut computed, decl);
            }
        }

        // 3. Inline style attribute (Origin: 2, highest priority)
        if let Some(style_attr) = dom.get_attribute(node_id, "style") {
            let inline_decls = nirvana_css::parse_inline_declarations(style_attr);
            for decl in &inline_decls {
                apply_declaration(&mut computed, decl);
            }
        }

        style_tree.insert(node_id, computed);
    }

    let current_style = style_tree.get_style(node_id).cloned();
    for child in dom.children(node_id) {
        compute_node_styles(
            dom,
            child,
            current_style.as_ref(),
            author_sheets,
            style_tree,
        );
    }
}

fn apply_declaration(computed: &mut ComputedValues, decl: &Declaration) {
    match decl.name.as_str() {
        "display" => {
            if let PropertyValue::Keyword(k) = &decl.value {
                match k.to_ascii_lowercase().as_str() {
                    "block" => computed.display = Display::Block,
                    "inline" | "inline-block" => computed.display = Display::Inline,
                    "none" => computed.display = Display::None,
                    _ => {}
                }
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
        "margin-top" => apply_length(&mut computed.margin_top, &decl.value),
        "margin-right" => apply_length(&mut computed.margin_right, &decl.value),
        "margin-bottom" => apply_length(&mut computed.margin_bottom, &decl.value),
        "margin-left" => apply_length(&mut computed.margin_left, &decl.value),
        "padding-top" => apply_padding(&mut computed.padding_top, &decl.value),
        "padding-right" => apply_padding(&mut computed.padding_right, &decl.value),
        "padding-bottom" => apply_padding(&mut computed.padding_bottom, &decl.value),
        "padding-left" => apply_padding(&mut computed.padding_left, &decl.value),
        "border" | "border-width" => {
            if let PropertyValue::Px(p) = decl.value {
                computed.border_width = p;
            }
        }
        "width" => apply_length(&mut computed.width, &decl.value),
        "height" => apply_length(&mut computed.height, &decl.value),
        _ => {}
    }
}

fn apply_padding(target: &mut Length, value: &PropertyValue) {
    if let PropertyValue::Px(p) = value {
        *target = Length::Px(*p);
    }
}

fn apply_length(target: &mut Length, value: &PropertyValue) {
    match value {
        PropertyValue::Px(p) => *target = Length::Px(*p),
        PropertyValue::Auto => *target = Length::Auto,
        _ => {}
    }
}
