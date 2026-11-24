//! Web Storage API (localStorage and sessionStorage) per the WHATWG Storage standard.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{
    js_string, Context, JsArgs, JsObject, JsValue,
};
use boa_gc::{Finalize, Trace};

/// Shared origin-partitioned storage storage backend.
#[derive(Debug, Clone, Default)]
pub struct StorageBackend {
    pub local_entries: HashMap<String, Vec<(String, String)>>,
    pub session_entries: HashMap<String, Vec<(String, String)>>,
}

impl StorageBackend {
    /// Creates a new empty storage backend.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieves a mutable reference to the key-value list for given origin and storage type.
    pub fn get_entries_mut(
        &mut self,
        origin: &str,
        is_session: bool,
    ) -> &mut Vec<(String, String)> {
        let map = if is_session {
            &mut self.session_entries
        } else {
            &mut self.local_entries
        };
        map.entry(origin.to_string()).or_default()
    }

    /// Retrieves an immutable reference to the key-value list for given origin and storage type.
    pub fn get_entries(&self, origin: &str, is_session: bool) -> Option<&Vec<(String, String)>> {
        let map = if is_session {
            &self.session_entries
        } else {
            &self.local_entries
        };
        map.get(origin)
    }

    /// Clears all local and session storage across all origins.
    pub fn clear(&mut self) {
        self.local_entries.clear();
        self.session_entries.clear();
    }
}

/// Capture struct for Boa NativeFunctions managing Web Storage.
#[derive(Clone, Trace, Finalize)]
pub struct StorageCapture {
    #[unsafe_ignore_trace]
    pub backend: Rc<RefCell<StorageBackend>>,
    pub is_session: bool,
    #[unsafe_ignore_trace]
    pub current_origin: Rc<RefCell<String>>,
}

/// Creates a new `Storage` JavaScript object for either `localStorage` or `sessionStorage`.
pub fn create_storage_object(
    context: &mut Context,
    backend: Rc<RefCell<StorageBackend>>,
    is_session: bool,
    current_origin: Rc<RefCell<String>>,
) -> Result<JsObject, boa_engine::JsError> {
    let realm = context.realm().clone();

    let cap_get = StorageCapture {
        backend: backend.clone(),
        is_session,
        current_origin: current_origin.clone(),
    };
    let get_item_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, cap, ctx| {
            let key = args.get_or_undefined(0).to_string(ctx)?;
            let key_str = key.to_std_string_escaped();
            let origin = cap.current_origin.borrow().clone();
            let backend = cap.backend.borrow();
            if let Some(entries) = backend.get_entries(&origin, cap.is_session)
                && let Some((_, val)) = entries.iter().find(|(k, _)| k == &key_str)
            {
                return Ok(JsValue::from(js_string!(val.as_str())));
            }
            Ok(JsValue::null())
        },
        cap_get,
    );

    let cap_set = StorageCapture {
        backend: backend.clone(),
        is_session,
        current_origin: current_origin.clone(),
    };
    let set_item_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, cap, ctx| {
            let key = args.get_or_undefined(0).to_string(ctx)?;
            let key_str = key.to_std_string_escaped();
            let val = args.get_or_undefined(1).to_string(ctx)?;
            let val_str = val.to_std_string_escaped();
            let origin = cap.current_origin.borrow().clone();
            let mut backend = cap.backend.borrow_mut();
            let entries = backend.get_entries_mut(&origin, cap.is_session);
            if let Some(existing) = entries.iter_mut().find(|(k, _)| k == &key_str) {
                existing.1 = val_str;
            } else {
                entries.push((key_str, val_str));
            }
            Ok(JsValue::undefined())
        },
        cap_set,
    );

    let cap_rem = StorageCapture {
        backend: backend.clone(),
        is_session,
        current_origin: current_origin.clone(),
    };
    let remove_item_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, cap, ctx| {
            let key = args.get_or_undefined(0).to_string(ctx)?;
            let key_str = key.to_std_string_escaped();
            let origin = cap.current_origin.borrow().clone();
            let mut backend = cap.backend.borrow_mut();
            let entries = backend.get_entries_mut(&origin, cap.is_session);
            entries.retain(|(k, _)| k != &key_str);
            Ok(JsValue::undefined())
        },
        cap_rem,
    );

    let cap_clear = StorageCapture {
        backend: backend.clone(),
        is_session,
        current_origin: current_origin.clone(),
    };
    let clear_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, cap, _ctx| {
            let origin = cap.current_origin.borrow().clone();
            let mut backend = cap.backend.borrow_mut();
            let entries = backend.get_entries_mut(&origin, cap.is_session);
            entries.clear();
            Ok(JsValue::undefined())
        },
        cap_clear,
    );

    let cap_key = StorageCapture {
        backend: backend.clone(),
        is_session,
        current_origin: current_origin.clone(),
    };
    let key_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, args, cap, ctx| {
            let index_val = args.get_or_undefined(0).to_numeric_number(ctx)?;
            let index = index_val as usize;
            let origin = cap.current_origin.borrow().clone();
            let backend = cap.backend.borrow();
            if let Some(entries) = backend.get_entries(&origin, cap.is_session)
                && index < entries.len()
            {
                return Ok(JsValue::from(js_string!(entries[index].0.as_str())));
            }
            Ok(JsValue::null())
        },
        cap_key,
    );

    let cap_len = StorageCapture {
        backend,
        is_session,
        current_origin,
    };
    let length_fn = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, cap, _ctx| {
            let origin = cap.current_origin.borrow().clone();
            let backend = cap.backend.borrow();
            let len = backend
                .get_entries(&origin, cap.is_session)
                .map(|e| e.len())
                .unwrap_or(0);
            Ok(JsValue::from(len as i32))
        },
        cap_len,
    );

    let length_getter_js = length_fn.to_js_function(&realm);

    let storage_obj = ObjectInitializer::new(context)
        .function(get_item_fn, js_string!("getItem"), 1)
        .function(set_item_fn, js_string!("setItem"), 2)
        .function(remove_item_fn, js_string!("removeItem"), 1)
        .function(clear_fn, js_string!("clear"), 0)
        .function(key_fn, js_string!("key"), 1)
        .accessor(
            js_string!("length"),
            Some(length_getter_js),
            None,
            Attribute::READONLY,
        )
        .build();

    Ok(storage_obj)
}
