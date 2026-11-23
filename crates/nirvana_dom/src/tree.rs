//! Arena-backed DOM Tree with cycle prevention and traversal operations.

use nirvana_core::arena::Arena;
use crate::node::{ElementData, Node, NodeData, NodeId, NodeType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomError {
    NodeNotFound,
    CycleDetected,
    NotAnElement,
    CannotAppendToLeafNode,
    CannotAppendDocumentRoot,
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound => write!(f, "Node not found in DOM arena"),
            Self::CycleDetected => write!(f, "Cycle detected in DOM tree mutation"),
            Self::NotAnElement => write!(f, "Target node is not an element"),
            Self::CannotAppendToLeafNode => write!(f, "Cannot append children to a text or comment node"),
            Self::CannotAppendDocumentRoot => write!(f, "Cannot append document root as a child node"),
        }
    }
}

impl std::error::Error for DomError {}

#[derive(Debug, Clone)]
pub struct DomTree {
    arena: Arena<Node>,
    root: NodeId,
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DomTree {
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let root = arena.alloc(Node::new(NodeData::Document));
        Self { arena, root }
    }

    #[inline]
    pub fn root_id(&self) -> NodeId {
        self.root
    }

    pub fn create_element(&mut self, tag: &str) -> NodeId {
        self.arena
            .alloc(Node::new(NodeData::Element(ElementData::new(tag))))
    }

    pub fn create_text(&mut self, text: &str) -> NodeId {
        self.arena
            .alloc(Node::new(NodeData::Text(text.to_string())))
    }

    pub fn create_comment(&mut self, comment: &str) -> NodeId {
        self.arena
            .alloc(Node::new(NodeData::Comment(comment.to_string())))
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.arena.get(id)
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.arena.get(id)?.parent
    }

    pub fn node_type(&self, id: NodeId) -> Option<NodeType> {
        self.arena.get(id).map(|n| n.node_type())
    }

    pub fn children(&self, parent: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut curr = match self.arena.get(parent) {
            Some(node) => node.first_child,
            None => return result,
        };

        while let Some(child_id) = curr {
            result.push(child_id);
            curr = self.arena.get(child_id).and_then(|n| n.next_sibling);
        }

        result
    }

    pub fn set_attribute(&mut self, id: NodeId, name: &str, value: &str) -> Result<(), DomError> {
        let node = self.arena.get_mut(id).ok_or(DomError::NodeNotFound)?;
        match &mut node.data {
            NodeData::Element(data) => {
                data.set_attribute(name, value);
                Ok(())
            }
            _ => Err(DomError::NotAnElement),
        }
    }

    pub fn get_attribute(&self, id: NodeId, name: &str) -> Option<&str> {
        let node = self.arena.get(id)?;
        match &node.data {
            NodeData::Element(data) => data.get_attribute(name),
            _ => None,
        }
    }

    /// Appends `child` as the last child of `parent`.
    /// Performs cycle prevention checks and updates doubly-linked sibling pointers.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DomError> {
        if parent == child {
            return Err(DomError::CycleDetected);
        }

        if child == self.root
            || matches!(
                self.arena.get(child).map(|n| &n.data),
                Some(NodeData::Document)
            )
        {
            return Err(DomError::CannotAppendDocumentRoot);
        }

        // Verify parent and child exist
        let parent_node = self.arena.get(parent).ok_or(DomError::NodeNotFound)?;
        if matches!(parent_node.data, NodeData::Text(_) | NodeData::Comment(_)) {
            return Err(DomError::CannotAppendToLeafNode);
        }

        if self.arena.get(child).is_none() {
            return Err(DomError::NodeNotFound);
        }

        // Cycle check: verify child is not an ancestor of parent
        let mut ancestor = Some(parent);
        while let Some(anc_id) = ancestor {
            if anc_id == child {
                return Err(DomError::CycleDetected);
            }
            ancestor = self.arena.get(anc_id).and_then(|n| n.parent);
        }

        // Detach child from old parent/siblings if it was attached anywhere
        self.detach_node(child)?;

        // Attach child to parent
        let old_last_child = self
            .arena
            .get(parent)
            .and_then(|p| p.last_child);

        if let Some(last_id) = old_last_child {
            // Update old last child's next_sibling to child
            if let Some(last_node) = self.arena.get_mut(last_id) {
                last_node.next_sibling = Some(child);
            }
            // Update child's prev_sibling to old last child
            if let Some(child_node) = self.arena.get_mut(child) {
                child_node.prev_sibling = Some(last_id);
                child_node.next_sibling = None;
                child_node.parent = Some(parent);
            }
            // Update parent's last_child
            if let Some(parent_node) = self.arena.get_mut(parent) {
                parent_node.last_child = Some(child);
            }
        } else {
            // First child of parent
            if let Some(child_node) = self.arena.get_mut(child) {
                child_node.prev_sibling = None;
                child_node.next_sibling = None;
                child_node.parent = Some(parent);
            }
            if let Some(parent_node) = self.arena.get_mut(parent) {
                parent_node.first_child = Some(child);
                parent_node.last_child = Some(child);
            }
        }

        Ok(())
    }

    /// Detaches a node from its current parent and sibling links.
    fn detach_node(&mut self, node_id: NodeId) -> Result<(), DomError> {
        let (parent, prev, next) = match self.arena.get(node_id) {
            Some(n) => (n.parent, n.prev_sibling, n.next_sibling),
            None => return Err(DomError::NodeNotFound),
        };

        if let Some(parent_node) = parent.and_then(|id| self.arena.get_mut(id)) {
            if parent_node.first_child == Some(node_id) {
                parent_node.first_child = next;
            }
            if parent_node.last_child == Some(node_id) {
                parent_node.last_child = prev;
            }
        }

        if let Some(prev_node) = prev.and_then(|id| self.arena.get_mut(id)) {
            prev_node.next_sibling = next;
        }

        if let Some(next_node) = next.and_then(|id| self.arena.get_mut(id)) {
            next_node.prev_sibling = prev;
        }

        if let Some(node) = self.arena.get_mut(node_id) {
            node.parent = None;
            node.prev_sibling = None;
            node.next_sibling = None;
        }

        Ok(())
    }
}
