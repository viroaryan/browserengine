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
use crate::storage::{create_storage_object, StorageBackend};
use crate::timers::{init_timers, TimerQueue};

#[derive(Clone, Trace, Finalize)]
struct FlagCapture {
    #[unsafe_ignore_trace]
    flag: Rc<RefCell<bool>>,
}

pub struct JsRuntime {
    context: Context,
    log_store: LogStore,
    timer_queue: TimerQueue,
    dom_handle: Option<DomHandle>,
    storage_backend: Rc<RefCell<StorageBackend>>,
    current_origin: Rc<RefCell<String>>,
    current_url: Rc<RefCell<String>>,
    cookie_jar: Rc<RefCell<nirvana_net::cookie::CookieJar>>,
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        Self::with_storage_and_cookies(
            Rc::new(RefCell::new(StorageBackend::new())),
            "null",
            Rc::new(RefCell::new(nirvana_net::cookie::CookieJar::new())),
            "about:blank",
        )
    }

    pub fn with_storage_and_cookies(
        storage_backend: Rc<RefCell<StorageBackend>>,
        origin: &str,
        cookie_jar: Rc<RefCell<nirvana_net::cookie::CookieJar>>,
        url: &str,
    ) -> Self {
        let mut context = Context::default();
        context.runtime_limits_mut().set_loop_iteration_limit(200_000);
        context.runtime_limits_mut().set_recursion_limit(256);

        let log_store: LogStore = Arc::new(Mutex::new(Vec::new()));
        let timer_queue = TimerQueue::new();
        let current_origin = Rc::new(RefCell::new(origin.to_string()));
        let current_url = Rc::new(RefCell::new(url.to_string()));

        let _ = init_console(&mut context, log_store.clone());
        let _ = init_timers(&mut context, timer_queue.clone());
        init_storage(&mut context, storage_backend.clone(), current_origin.clone());
        install_browser_shims(&mut context);

        Self {
            context,
            log_store,
            timer_queue,
            dom_handle: None,
            storage_backend,
            current_origin,
            current_url,
            cookie_jar,
        }
    }

    pub fn with_dom(dom: SharedDom) -> Self {
        let mut runtime = Self::new();
        let _ = runtime.bind_dom(dom);
        runtime
    }

    pub fn with_dom_and_storage(
        dom: SharedDom,
        storage_backend: Rc<RefCell<StorageBackend>>,
        origin: &str,
    ) -> Self {
        let mut runtime = Self::with_storage_and_cookies(
            storage_backend,
            origin,
            Rc::new(RefCell::new(nirvana_net::cookie::CookieJar::new())),
            "about:blank",
        );
        let _ = runtime.bind_dom(dom);
        runtime
    }

    pub fn with_full_context(
        dom: SharedDom,
        storage_backend: Rc<RefCell<StorageBackend>>,
        origin: &str,
        cookie_jar: Rc<RefCell<nirvana_net::cookie::CookieJar>>,
        url: &str,
    ) -> Self {
        let mut runtime = Self::with_storage_and_cookies(
            storage_backend,
            origin,
            cookie_jar,
            url,
        );
        let _ = runtime.bind_dom(dom);
        runtime
    }

    pub fn bind_dom(&mut self, dom: SharedDom) -> Result<(), String> {
        let handle = DomHandle::new_with_cookies(
            dom,
            self.cookie_jar.clone(),
            self.current_url.clone(),
        );
        init_dom(&mut self.context, handle.clone()).map_err(|e| format!("{e}"))?;
        self.dom_handle = Some(handle);
        Ok(())
    }

    pub fn set_origin(&mut self, origin: &str) {
        *self.current_origin.borrow_mut() = origin.to_string();
    }

    pub fn origin(&self) -> String {
        self.current_origin.borrow().clone()
    }

    pub fn set_url(&mut self, url_str: &str) {
        *self.current_url.borrow_mut() = url_str.to_string();
        if let Ok(url) = nirvana_url::Url::parse(url_str) {
            let origin = nirvana_net::security::Origin::from_url(&url).as_string();
            self.set_origin(&origin);
        }
    }

    pub fn url(&self) -> String {
        self.current_url.borrow().clone()
    }

    pub fn cookie_jar(&self) -> Rc<RefCell<nirvana_net::cookie::CookieJar>> {
        self.cookie_jar.clone()
    }

    pub fn set_cookie_jar(&mut self, jar: Rc<RefCell<nirvana_net::cookie::CookieJar>>) {
        self.cookie_jar = jar.clone();
        if let Some(ref mut handle) = self.dom_handle {
            handle.cookie_jar = jar;
        }
    }

    pub fn storage_backend(&self) -> Rc<RefCell<StorageBackend>> {
        self.storage_backend.clone()
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        let source = Source::from_bytes(code.as_bytes());
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
        }));
        match outcome {
            Ok(res) => res,
            Err(_) => Err("JS execution aborted safely (runtime panic recovered)".to_string()),
        }
    }

    pub fn step_timers(&mut self, delta_ms: u64) -> usize {
        let mut to_fire = Vec::new();

        {
            let mut timers = self.timer_queue.timers.borrow_mut();
            let mut i = 0;
            while i < timers.len() {
                if timers[i].remaining_ms <= delta_ms {
                    to_fire.push(timers[i].callback.clone());
                    if timers[i].is_interval {
                        timers[i].remaining_ms = timers[i].delay_ms;
                        i += 1;
                    } else {
                        timers.remove(i);
                    }
                } else {
                    timers[i].remaining_ms -= delta_ms;
                    i += 1;
                }
            }
        }

        let count = to_fire.len();
        for cb in to_fire {
            if cb.is_callable() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = cb.call(&JsValue::undefined(), &[], &mut self.context);
                }));
            }
        }

        count
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
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let _ = listener.call(
                            &curr_elem_obj.clone().into(),
                            &[event_obj.clone().into()],
                            &mut self.context,
                        );
                    }));
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

impl std::fmt::Debug for JsRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsRuntime")
            .field("logs", &self.logs().len())
            .field("has_dom", &self.dom_handle.is_some())
            .finish()
    }
}

fn init_storage(
    context: &mut Context,
    backend: Rc<RefCell<StorageBackend>>,
    origin: Rc<RefCell<String>>,
) {
    if let Ok(local) = create_storage_object(context, backend.clone(), false, origin.clone()) {
        let _ = context.register_global_property(
            js_string!("localStorage"),
            local.clone(),
            Attribute::all(),
        );
        let global = context.global_object();
        if let Ok(win) = global.get(js_string!("window"), context)
            && let Some(win_obj) = win.as_object()
        {
            let _ = win_obj.set(js_string!("localStorage"), local, false, context);
        }
    }
    if let Ok(session) = create_storage_object(context, backend, true, origin) {
        let _ = context.register_global_property(
            js_string!("sessionStorage"),
            session.clone(),
            Attribute::all(),
        );
        let global = context.global_object();
        if let Ok(win) = global.get(js_string!("window"), context)
            && let Some(win_obj) = win.as_object()
        {
            let _ = win_obj.set(js_string!("sessionStorage"), session, false, context);
        }
    }
}

fn install_browser_shims(context: &mut Context) {
    // Provide clean JS Web-API shims and replace Array.prototype.sort with a pure-JS
    // merge/insertion sort so non-transitive shuffles like () => 0.5 - Math.random()
    // never trigger Rust's slice::sort_by total-order panic.
    let shim_src = r#"
    (function() {
        if (typeof globalThis.window === 'undefined') {
            globalThis.window = globalThis;
        }
        var g = globalThis;
        var origSort = Array.prototype.sort;
        Array.prototype.sort = function(cmp) {
            if (typeof cmp !== 'function') {
                return origSort.call(this);
            }
            var O = Object(this);
            var len = O.length >>> 0;
            if (len <= 1) return this;
            var items = [];
            for (var i = 0; i < len; i++) {
                if (i in O) items.push(O[i]);
            }
            var n = items.length;
            if (n <= 32) {
                for (var i = 1; i < n; i++) {
                    var key = items[i];
                    var j = i - 1;
                    while (j >= 0 && Number(cmp(items[j], key)) > 0) {
                        items[j + 1] = items[j];
                        j--;
                    }
                    items[j + 1] = key;
                }
            } else {
                var tmp = new Array(n);
                for (var width = 1; width < n; width *= 2) {
                    for (var left = 0; left < n; left += 2 * width) {
                        var mid = Math.min(left + width, n);
                        var right = Math.min(left + 2 * width, n);
                        var l = left, r = mid, k = left;
                        while (l < mid && r < right) {
                            if (Number(cmp(items[l], items[r])) <= 0) {
                                tmp[k++] = items[l++];
                            } else {
                                tmp[k++] = items[r++];
                            }
                        }
                        while (l < mid) tmp[k++] = items[l++];
                        while (r < right) tmp[k++] = items[r++];
                    }
                    for (var c = 0; c < n; c++) items[c] = tmp[c];
                }
            }
            for (var k = 0; k < n; k++) O[k] = items[k];
            for (var k = n; k < len; k++) delete O[k];
            return this;
        };

        if (typeof g.navigator === 'undefined') {
            g.navigator = {
                userAgent: 'Mozilla/5.0 (Android 14; Mobile; rv:132.0) Gecko/132.0 Firefox/132.0',
                language: 'en-US',
                languages: ['en-US', 'en'],
                onLine: true,
                cookieEnabled: true,
                platform: 'Linux aarch64'
            };
        }
        if (typeof g.performance === 'undefined') {
            var startMs = Date.now();
            g.performance = { now: function() { return Date.now() - startMs; }, timing: {} };
        }
        if (typeof g.requestAnimationFrame !== 'function') {
            g.requestAnimationFrame = function(cb) { return setTimeout(cb, 16); };
            g.cancelAnimationFrame = function(id) { clearTimeout(id); };
        }
        if (typeof g.matchMedia !== 'function') {
            g.matchMedia = function(q) {
                return {
                    matches: false,
                    media: String(q || ''),
                    addListener: function() {},
                    removeListener: function() {},
                    addEventListener: function() {},
                    removeEventListener: function() {}
                };
            };
        }
        if (typeof g.getComputedStyle !== 'function') {
            g.getComputedStyle = function(el) {
                return (el && el.style) ? el.style : {};
            };
        }
        if (typeof g.IntersectionObserver === 'undefined') {
            g.IntersectionObserver = function() {
                this.observe = function() {};
                this.unobserve = function() {};
                this.disconnect = function() {};
            };
        }
        if (typeof g.ResizeObserver === 'undefined') {
            g.ResizeObserver = function() {
                this.observe = function() {};
                this.unobserve = function() {};
                this.disconnect = function() {};
            };
        }
        if (typeof g.MutationObserver === 'undefined') {
            g.MutationObserver = function() {
                this.observe = function() {};
                this.disconnect = function() {};
                this.takeRecords = function() { return []; };
            };
        }
        if (typeof g.scrollTo !== 'function') g.scrollTo = function() {};
        if (typeof g.scroll !== 'function') g.scroll = function() {};
    })();
    "#;
    let _ = context.eval(Source::from_bytes(shim_src.as_bytes()));
}
