use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use nirvana_dom::NodeId;
use boa_engine::{
    js_string,
    native_function::NativeFunction,
    object::ObjectInitializer,
    property::Attribute,
    Context, JsValue, Source,
};
use boa_gc::{Finalize, Trace};
use crate::console::{init_console, LogEntry, LogStore};
use crate::dom_binding::{create_element_object, init_dom, DomHandle, SharedDom};

#[derive(Clone, Trace, Finalize)]
struct FlagCapture {
    #[unsafe_ignore_trace]
    flag: Rc<RefCell<bool>>,
}

pub struct JsRuntime {
    context: Context,
    log_store: LogStore,
    dom_handle: Option<DomHandle>,
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        let mut context = Context::default();
        let log_store: LogStore = Arc::new(Mutex::new(Vec::new()));

        let _ = init_console(&mut context, log_store.clone());

        Self {
            context,
            log_store,
            dom_handle: None,
        }
    }

    pub fn with_dom(dom: SharedDom) -> Self {
        let mut context = Context::default();
        let log_store: LogStore = Arc::new(Mutex::new(Vec::new()));

        let _ = init_console(&mut context, log_store.clone());
        let handle = DomHandle::new(dom);
        let _ = init_dom(&mut context, handle.clone());

        Self {
            context,
            log_store,
            dom_handle: Some(handle),
        }
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        let source = Source::from_bytes(code.as_bytes());
        match self.context.eval(source) {
            Ok(val) => {
                let res = val
                    .to_string(&mut self.context)
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_else(|e| format!("[Conversion Error: {e}]"));
                Ok(res)
            }
            Err(err) => Err(format!("{err}")),
        }
    }

    pub fn dispatch_event(&mut self, target_node: NodeId, event_type: &str) -> Result<bool, String> {
        let Some(ref handle) = self.dom_handle else {
            return Ok(true);
        };

        // 1. Build list of bubble targets from target_node to root
        let mut bubble_chain = Vec::new();
        {
            let dom = handle.dom.borrow();
            let mut curr = Some(target_node);
            while let Some(node_id) = curr {
                bubble_chain.push(node_id);
                curr = dom.parent(node_id);
            }
        }

        let target_obj = create_element_object(&mut self.context, handle.clone(), target_node)
            .map_err(|e| format!("{e}"))?;

        let stopped_flag = Rc::new(RefCell::new(false));
        let s_capture = FlagCapture { flag: stopped_flag.clone() };
        let stop_prop_fn = NativeFunction::from_copy_closure_with_captures(
            |_this, _args, cap, _ctx| {
                *cap.flag.borrow_mut() = true;
                Ok(JsValue::undefined())
            },
            s_capture,
        );

        let prevented_flag = Rc::new(RefCell::new(false));
        let p_capture = FlagCapture { flag: prevented_flag.clone() };
        let prevent_def_fn = NativeFunction::from_copy_closure_with_captures(
            |_this, _args, cap, _ctx| {
                *cap.flag.borrow_mut() = true;
                Ok(JsValue::undefined())
            },
            p_capture,
        );

        let event_obj = ObjectInitializer::new(&mut self.context)
            .property(js_string!("type"), js_string!(event_type), Attribute::all())
            .property(js_string!("target"), target_obj.clone(), Attribute::all())
            .property(js_string!("currentTarget"), target_obj.clone(), Attribute::all())
            .function(stop_prop_fn, js_string!("stopPropagation"), 0)
            .function(prevent_def_fn, js_string!("preventDefault"), 0)
            .build();

        for node_id in bubble_chain {
            let listeners = handle.events.get_listeners(node_id, event_type);
            if listeners.is_empty() {
                continue;
            }

            let curr_elem_obj = create_element_object(&mut self.context, handle.clone(), node_id)
                .map_err(|e| format!("{e}"))?;
            let _ = event_obj.set(
                js_string!("currentTarget"),
                curr_elem_obj.clone(),
                false,
                &mut self.context,
            );

            for listener in listeners {
                if listener.is_callable() {
                    let _ = listener.call(
                        &curr_elem_obj.clone().into(),
                        &[event_obj.clone().into()],
                        &mut self.context,
                    );
                }
            }

            if *stopped_flag.borrow() {
                break;
            }
        }

        Ok(!*prevented_flag.borrow())
    }

    pub fn has_mutated(&self) -> bool {
        self.dom_handle
            .as_ref()
            .map(|h| *h.mutated.borrow())
            .unwrap_or(false)
    }

    pub fn clear_mutated(&self) {
        if let Some(ref h) = self.dom_handle {
            *h.mutated.borrow_mut() = false;
        }
    }

    pub fn logs(&self) -> Vec<LogEntry> {
        self.log_store.lock().map(|l| l.clone()).unwrap_or_default()
    }

    pub fn clear_logs(&self) {
        if let Ok(mut lock) = self.log_store.lock() {
            lock.clear();
        }
    }

    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }
}
