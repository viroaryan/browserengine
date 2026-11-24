use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use nirvana_dom::NodeId;
use boa_engine::JsObject;
use boa_gc::{Finalize, Trace};

pub type EventCallbackList = Vec<JsObject>;

#[derive(Default, Clone, Trace, Finalize)]
pub struct EventRegistry {
    #[unsafe_ignore_trace]
    pub listeners: Rc<RefCell<HashMap<(NodeId, String), EventCallbackList>>>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            listeners: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn add_listener(&self, node_id: NodeId, event_type: &str, callback: JsObject) {
        let mut map = self.listeners.borrow_mut();
        map.entry((node_id, event_type.to_ascii_lowercase()))
            .or_default()
            .push(callback);
    }

    pub fn get_listeners(&self, node_id: NodeId, event_type: &str) -> Vec<JsObject> {
        self.listeners
            .borrow()
            .get(&(node_id, event_type.to_ascii_lowercase()))
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        self.listeners.borrow_mut().clear();
    }
}
