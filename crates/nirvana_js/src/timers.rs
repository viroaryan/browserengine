use std::cell::RefCell;
use std::rc::Rc;
use boa_engine::{
    js_string,
    native_function::NativeFunction,
    Context, JsArgs, JsObject, JsResult, JsValue,
};
use boa_gc::{Finalize, Trace};

#[derive(Clone, Trace, Finalize)]
pub struct Timer {
    pub id: u32,
    pub callback: JsObject,
    pub delay_ms: u64,
    pub remaining_ms: u64,
    pub is_interval: bool,
}

#[derive(Default, Clone, Trace, Finalize)]
pub struct TimerQueue {
    #[unsafe_ignore_trace]
    pub next_id: Rc<RefCell<u32>>,
    #[unsafe_ignore_trace]
    pub timers: Rc<RefCell<Vec<Timer>>>,
}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            next_id: Rc::new(RefCell::new(1)),
            timers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn add_timer(&self, callback: JsObject, delay_ms: u64, is_interval: bool) -> u32 {
        let mut id_guard = self.next_id.borrow_mut();
        let id = *id_guard;
        *id_guard += 1;

        self.timers.borrow_mut().push(Timer {
            id,
            callback,
            delay_ms,
            remaining_ms: delay_ms,
            is_interval,
        });

        id
    }

    pub fn remove_timer(&self, id: u32) {
        let mut timers = self.timers.borrow_mut();
        timers.retain(|t| t.id != id);
    }

    pub fn clear(&self) {
        self.timers.borrow_mut().clear();
    }
}

pub fn init_timers(context: &mut Context, queue: TimerQueue) -> JsResult<()> {
    let q_timeout = queue.clone();
    let set_timeout = NativeFunction::from_copy_closure_with_captures(
        |_this, args, queue, ctx| {
            let callback_val = args.get_or_undefined(0);
            let Some(cb_obj) = callback_val.as_object() else {
                return Ok(JsValue::new(0));
            };
            let delay = args.get_or_undefined(1).to_number(ctx).unwrap_or(0.0).max(0.0) as u64;
            let id = queue.add_timer(cb_obj.clone(), delay, false);
            Ok(JsValue::new(id))
        },
        q_timeout,
    );

    let q_clear = queue.clone();
    let clear_timeout = NativeFunction::from_copy_closure_with_captures(
        |_this, args, queue, ctx| {
            let id = args.get_or_undefined(0).to_number(ctx).unwrap_or(0.0) as u32;
            queue.remove_timer(id);
            Ok(JsValue::undefined())
        },
        q_clear,
    );

    let q_interval = queue.clone();
    let set_interval = NativeFunction::from_copy_closure_with_captures(
        |_this, args, queue, ctx| {
            let callback_val = args.get_or_undefined(0);
            let Some(cb_obj) = callback_val.as_object() else {
                return Ok(JsValue::new(0));
            };
            let delay = args.get_or_undefined(1).to_number(ctx).unwrap_or(0.0).max(0.0) as u64;
            let id = queue.add_timer(cb_obj.clone(), delay, true);
            Ok(JsValue::new(id))
        },
        q_interval,
    );

    let q_clear_int = queue;
    let clear_interval = NativeFunction::from_copy_closure_with_captures(
        |_this, args, queue, ctx| {
            let id = args.get_or_undefined(0).to_number(ctx).unwrap_or(0.0) as u32;
            queue.remove_timer(id);
            Ok(JsValue::undefined())
        },
        q_clear_int,
    );

    let global = context.global_object();
    global.set(js_string!("setTimeout"), set_timeout.to_js_function(context.realm()), false, context)?;
    global.set(js_string!("clearTimeout"), clear_timeout.to_js_function(context.realm()), false, context)?;
    global.set(js_string!("setInterval"), set_interval.to_js_function(context.realm()), false, context)?;
    global.set(js_string!("clearInterval"), clear_interval.to_js_function(context.realm()), false, context)?;

    Ok(())
}
