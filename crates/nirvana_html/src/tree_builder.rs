//! HTML tree builder creating an arena-backed DOM tree from tokens.

use nirvana_dom::node::NodeData;
use nirvana_dom::{DomTree, NodeId};
use crate::token::Token;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

/// Maximum depth of nested open elements permitted in DOM tree construction (512 levels).
/// Prevents thread stack exhaustion on adversarial documents.
pub const MAX_TREE_DEPTH: usize = 512;

pub struct TreeBuilder {
    tokens: Vec<Token>,
    tree: DomTree,
    open_elements: Vec<NodeId>,
    has_html: bool,
    has_body: bool,
    text_buffer: String,
    excess_depth: usize,
}

impl TreeBuilder {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            tree: DomTree::new(),
            open_elements: Vec::new(),
            has_html: false,
            has_body: false,
            text_buffer: String::new(),
            excess_depth: 0,
        }
    }

    #[inline]
    fn is_void(tag_name: &str) -> bool {
        VOID_ELEMENTS.contains(&tag_name)
    }

    fn current_node(&self) -> NodeId {
        self.open_elements
            .last()
            .copied()
            .unwrap_or_else(|| self.tree.root_id())
    }

    fn flush_text(&mut self) {
        if !self.text_buffer.is_empty() {
            if self.open_elements.is_empty() {
                // Do not append trailing text to document root after </html> is closed
                self.text_buffer.clear();
                return;
            }
            let parent = self.current_node();
            let text_node = self.tree.create_text(&self.text_buffer);
            let _ = self.tree.append_child(parent, text_node);
            self.text_buffer.clear();
        }
    }

    fn is_inside_head(&self) -> bool {
        self.open_elements.iter().any(|&elem_id| {
            matches!(
                self.tree.get_node(elem_id).map(|n| &n.data),
                Some(NodeData::Element(data)) if data.tag_name == "head"
            )
        })
    }

    fn ensure_html_and_body(&mut self, next_tag: &str) {
        if !self.has_html && next_tag != "html" {
            let html = self.tree.create_element("html");
            let root = self.tree.root_id();
            let _ = self.tree.append_child(root, html);
            self.open_elements.push(html);
            self.has_html = true;
        }

        let is_head_tag = matches!(
            next_tag,
            "head" | "title" | "style" | "script" | "meta" | "link" | "base" | "noscript"
        );

        if !self.has_body
            && !self.is_inside_head()
            && !is_head_tag
            && next_tag != "html"
        {
            let html_node = self
                .open_elements
                .first()
                .copied()
                .unwrap_or_else(|| self.tree.root_id());
            let body = self.tree.create_element("body");
            let _ = self.tree.append_child(html_node, body);
            self.open_elements.push(body);
            self.has_body = true;
        }
    }

    pub fn build(mut self) -> DomTree {
        let tokens = std::mem::take(&mut self.tokens);

        for token in tokens {
            match token {
                Token::Doctype(_) => {
                    // Doctype processed, document already has Root Node
                }

                Token::StartTag(tag) => {
                    if self.excess_depth > 0 {
                        if !Self::is_void(&tag.name) && !tag.self_closing {
                            self.excess_depth += 1;
                        }
                        continue;
                    }

                    self.flush_text();

                    if tag.name == "html" {
                        if !self.has_html {
                            let html = self.tree.create_element("html");
                            for (k, v) in &tag.attributes {
                                let _ = self.tree.set_attribute(html, k, v);
                            }
                            let root = self.tree.root_id();
                            let _ = self.tree.append_child(root, html);
                            self.open_elements.push(html);
                            self.has_html = true;
                        } else if let Some(&html_node) = self.open_elements.first() {
                            for (k, v) in &tag.attributes {
                                let _ = self.tree.set_attribute(html_node, k, v);
                            }
                        }
                        continue;
                    }

                    if tag.name == "head" {
                        if !self.has_html {
                            let html = self.tree.create_element("html");
                            let root = self.tree.root_id();
                            let _ = self.tree.append_child(root, html);
                            self.open_elements.push(html);
                            self.has_html = true;
                        }
                        let html_node = self
                            .open_elements
                            .first()
                            .copied()
                            .unwrap_or_else(|| self.tree.root_id());
                        let head = self.tree.create_element("head");
                        for (k, v) in &tag.attributes {
                            let _ = self.tree.set_attribute(head, k, v);
                        }
                        let _ = self.tree.append_child(html_node, head);
                        self.open_elements.push(head);
                        continue;
                    }

                    if tag.name == "body" {
                        if !self.has_html {
                            let html = self.tree.create_element("html");
                            let root = self.tree.root_id();
                            let _ = self.tree.append_child(root, html);
                            self.open_elements.push(html);
                            self.has_html = true;
                        }
                        if !self.has_body {
                            let html_node = self
                                .open_elements
                                .first()
                                .copied()
                                .unwrap_or_else(|| self.tree.root_id());
                            let body = self.tree.create_element("body");
                            for (k, v) in &tag.attributes {
                                let _ = self.tree.set_attribute(body, k, v);
                            }
                            let _ = self.tree.append_child(html_node, body);
                            self.open_elements.push(body);
                            self.has_body = true;
                        } else {
                            // Merge attributes into existing body element
                            for &elem in &self.open_elements {
                                if matches!(
                                    self.tree.get_node(elem).map(|n| &n.data),
                                    Some(NodeData::Element(data)) if data.tag_name == "body"
                                ) {
                                    for (k, v) in &tag.attributes {
                                        let _ = self.tree.set_attribute(elem, k, v);
                                    }
                                    break;
                                }
                            }
                        }
                        continue;
                    }

                    self.ensure_html_and_body(&tag.name);

                    if self.open_elements.len() >= MAX_TREE_DEPTH {
                        if !Self::is_void(&tag.name) && !tag.self_closing {
                            self.excess_depth += 1;
                        }
                        continue;
                    }

                    let elem = self.tree.create_element(&tag.name);
                    for (k, v) in &tag.attributes {
                        let _ = self.tree.set_attribute(elem, k, v);
                    }

                    let parent = self.current_node();
                    let _ = self.tree.append_child(parent, elem);

                    if !Self::is_void(&tag.name) && !tag.self_closing {
                        self.open_elements.push(elem);
                    }
                }

                Token::EndTag(tag) => {
                    if self.excess_depth > 0 {
                        self.excess_depth -= 1;
                        continue;
                    }

                    self.flush_text();

                    // Find matching open element in stack from the top
                    let found_index = self.open_elements.iter().rposition(|&elem_id| {
                        matches!(
                            self.tree.get_node(elem_id).map(|n| &n.data),
                            Some(NodeData::Element(data)) if data.tag_name == tag.name
                        )
                    });

                    if let Some(idx) = found_index {
                        // Pop elements down to and including idx
                        self.open_elements.truncate(idx);
                    }
                }

                Token::Character(c) => {
                    if self.excess_depth > 0 {
                        continue;
                    }
                    // WHATWG: Ignore whitespace characters if not inside body or text-bearing element
                    if c.is_whitespace() {
                        let curr = self.current_node();
                        if curr == self.tree.root_id()
                            || matches!(
                                self.tree.get_node(curr).map(|n| &n.data),
                                Some(NodeData::Element(data)) if data.tag_name == "html"
                            )
                        {
                            continue;
                        }
                    }
                    self.ensure_html_and_body("p");
                    self.text_buffer.push(c);
                }

                Token::Comment(comment) => {
                    if self.excess_depth > 0 {
                        continue;
                    }
                    self.flush_text();
                    let comment_node = self.tree.create_comment(&comment);
                    let parent = self.current_node();
                    let _ = self.tree.append_child(parent, comment_node);
                }

                Token::Eof => {
                    self.flush_text();
                    self.excess_depth = 0;
                    self.open_elements.clear();
                }
            }
        }

        self.flush_text();
        self.tree
    }
}
