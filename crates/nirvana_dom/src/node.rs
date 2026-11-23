//! DOM node representation and data types.

use nirvana_core::arena::Id;

pub type NodeId = Id<Node>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Document,
    Element,
    Text,
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
}

impl ElementData {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_ascii_lowercase(),
            attributes: Vec::new(),
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_ascii_lowercase();
        self.attributes
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        let name_lower = name.to_ascii_lowercase();
        if let Some(existing) = self
            .attributes
            .iter_mut()
            .find(|(k, _)| k.to_ascii_lowercase() == name_lower)
        {
            existing.1 = value.to_string();
        } else {
            self.attributes.push((name.to_string(), value.to_string()));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeData {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub data: NodeData,
}

impl Node {
    pub fn new(data: NodeData) -> Self {
        Self {
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            data,
        }
    }

    pub fn node_type(&self) -> NodeType {
        match self.data {
            NodeData::Document => NodeType::Document,
            NodeData::Element(_) => NodeType::Element,
            NodeData::Text(_) => NodeType::Text,
            NodeData::Comment(_) => NodeType::Comment,
        }
    }
}
