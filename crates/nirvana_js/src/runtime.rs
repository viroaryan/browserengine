use std::sync::{Arc, Mutex};
use boa_engine::{Context, Source};
use crate::console::{init_console, LogEntry, LogStore};
use crate::dom_binding::{init_dom, DomHandle, SharedDom};

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
