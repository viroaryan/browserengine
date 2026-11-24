use std::cell::RefCell;
use std::rc::Rc;
use nirvana_dom::{DomTree, NodeData, NodeId, NodeType};
use nirvana_html::parse_html;
use boa_engine::{
    js_string,
    native_function::NativeFunction,
    object::ObjectInitializer,
    property::Attribute,
    Context, JsArgs, JsObject, JsResult, JsValue, Source,
};
use boa_gc::{Finalize, Trace};

pub type SharedDom = Rc<RefCell<DomTree>>;

#[derive(Clone, Trace, Finalize)]
pub struct DomHandle {
    #[unsafe_ignore_trace]
    pub dom: SharedDom,
    #[unsafe_ignore_trace]
    pub mutated: Rc<RefCell<bool>>,
    pub events: crate::events::EventRegistry,
}

impl DomHandle {
    pub fn new(dom: SharedDom) -> Self {
        Self {
            dom,
            mutated: Rc::new(RefCell::new(false)),
            events: crate::events::EventRegistry::new(),
        }
    }
}

pub fn init_dom(context: &mut Context, handle: DomHandle) -> JsResult<()> {
    // 1. Setup window global reference
    let global = context.global_object();
    global.set(js_string!("window"), global.clone(), false, context)?;

    // 2. Install __createStyleProxy helper
    let proxy_script = r#"
        globalThis.__createStyleProxy = function(target) {
            return new Proxy(target, {
                get(t, p) {
                    if (typeof p === 'string' && !(p in t)) {
                        return t.getProperty(p);
                    }
                    return t[p];
                },
                set(t, p, v) {
                    if (typeof p === 'string') {
                        t.setProperty(p, String(v));
                        return true;
                    }
                    t[p] = v;
                    return true;
                }
            });
        };
    "#;
    let _ = context.eval(Source::from_bytes(proxy_script.as_bytes()));

    // 3. Document object
    let h_get_id = handle.clone();
    let get_element_by_id = NativeFunction::from_copy_closure_with_captures(
        |_this, args, handle, ctx| {
            let id_str = args.get_or_undefined(0).to_string(ctx)?;
            let id_str = id_str.to_std_string_escaped();
            let dom = handle.dom.borrow();
            let mut found = None;
            for node_id in dom.nodes() {
                if dom.node_type(node_id) == Some(NodeType::Element)
                    && dom.get_attribute(node_id, "id") == Some(&id_str)
                {
                    found = Some(node_id);
                    break;
                }
            }
            drop(dom);
            match found {
                Some(node_id) => Ok(create_element_object(ctx, handle.clone(), node_id)?.into()),
                None => Ok(JsValue::null()),
            }
        },
        h_get_id,
    );

    let h_query = handle.clone();
    let query_selector = NativeFunction::from_copy_closure_with_captures(
        |_this, args, handle, ctx| {
            let sel = args.get_or_undefined(0).to_string(ctx)?;
            let sel = sel.to_std_string_escaped();
            let dom = handle.dom.borrow();
            let mut found = None;
            for node_id in dom.nodes() {
                if dom.node_type(node_id) == Some(NodeType::Element) {
                    let matches = if let Some(id) = sel.strip_prefix('#') {
                        dom.get_attribute(node_id, "id") == Some(id)
                    } else if let Some(class) = sel.strip_prefix('.') {
                        dom.get_attribute(node_id, "class")
                            .map(|c| c.split_whitespace().any(|cls| cls == class))
                            .unwrap_or(false)
                    } else {
                        dom.tag_name(node_id)
                            .map(|t| t.eq_ignore_ascii_case(&sel))
                            .unwrap_or(false)
                    };
                    if matches {
                        found = Some(node_id);
                        break;
                    }
                }
            }
            drop(dom);
            match found {
                Some(node_id) => Ok(create_element_object(ctx, handle.clone(), node_id)?.into()),
                None => Ok(JsValue::null()),
            }
        },
        h_query,
    );

    let h_create = handle.clone();
    let create_element = NativeFunction::from_copy_closure_with_captures(
        |_this, args, handle, ctx| {
            let tag = args.get_or_undefined(0).to_string(ctx)?;
            let tag = tag.to_std_string_escaped();
            let node_id = handle.dom.borrow_mut().create_element(&tag);
            Ok(create_element_object(ctx, handle.clone(), node_id)?.into())
        },
        h_create,
    );

    let h_title_get = handle.clone();
    let get_title = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, handle, _ctx| {
            let dom = handle.dom.borrow();
            for n in dom.nodes() {
                if dom.tag_name(n).map(|t| t.eq_ignore_ascii_case("title")).unwrap_or(false) {
                    let text = collect_text_content(&dom, n);
                    return Ok(js_string!(text).into());
                }
            }
            Ok(js_string!("").into())
        },
        h_title_get,
    );

    let h_title_set = handle.clone();
    let set_title = NativeFunction::from_copy_closure_with_captures(
        |_this, args, handle, ctx| {
            let text = args.get_or_undefined(0).to_string(ctx)?;
            let text = text.to_std_string_escaped();
            let title_node = {
                let dom = handle.dom.borrow();
                dom.nodes().find(|&n| {
                    dom.tag_name(n)
                        .map(|t| t.eq_ignore_ascii_case("title"))
                        .unwrap_or(false)
                })
            };
            if let Some(n) = title_node {
                let mut dom = handle.dom.borrow_mut();
                set_text_content(&mut dom, n, &text);
                *handle.mutated.borrow_mut() = true;
            }
            Ok(JsValue::undefined())
        },
        h_title_set,
    );

    let h_body_get = handle.clone();
    let get_body = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, handle, ctx| {
            let body_node = {
                let dom = handle.dom.borrow();
                dom.nodes().find(|&n| {
                    dom.tag_name(n)
                        .map(|t| t.eq_ignore_ascii_case("body"))
                        .unwrap_or(false)
                })
            };
            if let Some(n) = body_node {
                Ok(create_element_object(ctx, handle.clone(), n)?.into())
            } else {
                Ok(JsValue::null())
            }
        },
        h_body_get,
    );

    let get_title_fn = get_title.to_js_function(context.realm());
    let set_title_fn = set_title.to_js_function(context.realm());
    let get_body_fn = get_body.to_js_function(context.realm());

    let document = ObjectInitializer::new(context)
        .function(get_element_by_id, js_string!("getElementById"), 1)
        .function(query_selector, js_string!("querySelector"), 1)
        .function(create_element, js_string!("createElement"), 1)
        .accessor(
            js_string!("title"),
            Some(get_title_fn),
            Some(set_title_fn),
            Attribute::all(),
        )
        .accessor(
            js_string!("body"),
            Some(get_body_fn),
            None,
            Attribute::READONLY,
        )
        .build();

    global.set(js_string!("document"), document, false, context)?;
    Ok(())
}

pub fn create_element_object(
    context: &mut Context,
    handle: DomHandle,
    node_id: NodeId,
) -> JsResult<JsObject> {
    let raw_id = node_id.raw() as f64;

    // getAttribute
    let h_get_attr = handle.clone();
    let get_attr_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::null());
            };
            let attr_name = args.get_or_undefined(0).to_string(ctx)?;
            let attr_name = attr_name.to_std_string_escaped();
            let dom = handle.dom.borrow();
            match dom.get_attribute(elem_id, &attr_name) {
                Some(val) => Ok(js_string!(val).into()),
                None => Ok(JsValue::null()),
            }
        },
        h_get_attr,
    );

    // setAttribute
    let h_set_attr = handle.clone();
    let set_attr_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let attr_name = args.get_or_undefined(0).to_string(ctx)?;
            let attr_name = attr_name.to_std_string_escaped();
            let attr_val = args.get_or_undefined(1).to_string(ctx)?;
            let attr_val = attr_val.to_std_string_escaped();
            let mut dom = handle.dom.borrow_mut();
            let _ = dom.set_attribute(elem_id, &attr_name, &attr_val);
            *handle.mutated.borrow_mut() = true;
            Ok(JsValue::undefined())
        },
        h_set_attr,
    );

    // removeAttribute
    let h_rem_attr = handle.clone();
    let rem_attr_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let attr_name = args.get_or_undefined(0).to_string(ctx)?;
            let attr_name = attr_name.to_std_string_escaped();
            let mut dom = handle.dom.borrow_mut();
            let _ = dom.remove_attribute(elem_id, &attr_name);
            *handle.mutated.borrow_mut() = true;
            Ok(JsValue::undefined())
        },
        h_rem_attr,
    );

    // textContent getter/setter
    let h_tc_get = handle.clone();
    let get_tc_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(js_string!("").into());
            };
            let dom = handle.dom.borrow();
            let text = collect_text_content(&dom, elem_id);
            Ok(js_string!(text).into())
        },
        h_tc_get,
    );

    let h_tc_set = handle.clone();
    let set_tc_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let text = args.get_or_undefined(0).to_string(ctx)?;
            let text = text.to_std_string_escaped();
            let mut dom = handle.dom.borrow_mut();
            set_text_content(&mut dom, elem_id, &text);
            *handle.mutated.borrow_mut() = true;
            Ok(JsValue::undefined())
        },
        h_tc_set,
    );

    // innerHTML getter/setter
    let h_ih_get = handle.clone();
    let get_ih_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(js_string!("").into());
            };
            let dom = handle.dom.borrow();
            let html = serialize_inner_html(&dom, elem_id);
            Ok(js_string!(html).into())
        },
        h_ih_get,
    );

    let h_ih_set = handle.clone();
    let set_ih_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let html = args.get_or_undefined(0).to_string(ctx)?;
            let html = html.to_std_string_escaped();
            let mut dom = handle.dom.borrow_mut();
            set_inner_html(&mut dom, elem_id, &html);
            *handle.mutated.borrow_mut() = true;
            Ok(JsValue::undefined())
        },
        h_ih_set,
    );

    // value getter/setter
    let h_val_get = handle.clone();
    let get_val_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(js_string!("").into());
            };
            let dom = handle.dom.borrow();
            let val = dom.get_attribute(elem_id, "value").unwrap_or("");
            Ok(js_string!(val).into())
        },
        h_val_get,
    );

    let h_val_set = handle.clone();
    let set_val_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let val = args.get_or_undefined(0).to_string(ctx)?;
            let val = val.to_std_string_escaped();
            let mut dom = handle.dom.borrow_mut();
            let _ = dom.set_attribute(elem_id, "value", &val);
            *handle.mutated.borrow_mut() = true;
            Ok(JsValue::undefined())
        },
        h_val_set,
    );

    // tagName getter
    let h_tag_get = handle.clone();
    let get_tag_fn = NativeFunction::from_copy_closure_with_captures(
        |this, _args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(js_string!("").into());
            };
            let dom = handle.dom.borrow();
            let tag = dom.tag_name(elem_id).unwrap_or("").to_ascii_uppercase();
            Ok(js_string!(tag).into())
        },
        h_tag_get,
    );

    // appendChild
    let h_append = handle.clone();
    let append_child_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(parent_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::null());
            };
            let child_val = args.get_or_undefined(0);
            let Some(child_id) = get_node_id_from_this(child_val, ctx) else {
                return Ok(JsValue::null());
            };
            let mut dom = handle.dom.borrow_mut();
            let _ = dom.append_child(parent_id, child_id);
            *handle.mutated.borrow_mut() = true;
            Ok(child_val.clone())
        },
        h_append,
    );

    // Style backing object
    let h_style_get = handle.clone();
    let style_get_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(js_string!("").into());
            };
            let prop = args.get_or_undefined(0).to_string(ctx)?;
            let prop = prop.to_std_string_escaped();
            let dom = handle.dom.borrow();
            let val = get_style_property(&dom, elem_id, &prop);
            Ok(js_string!(val).into())
        },
        h_style_get,
    );

    let h_style_set = handle.clone();
    let style_set_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let prop = args.get_or_undefined(0).to_string(ctx)?;
            let prop = prop.to_std_string_escaped();
            let val = args.get_or_undefined(1).to_string(ctx)?;
            let val = val.to_std_string_escaped();
            set_style_property(&mut handle.dom.borrow_mut(), elem_id, &prop, &val);
            *handle.mutated.borrow_mut() = true;
            Ok(JsValue::undefined())
        },
        h_style_set,
    );

    // addEventListener
    let h_add_ev = handle.clone();
    let add_event_listener_fn = NativeFunction::from_copy_closure_with_captures(
        |this, args, handle, ctx| {
            let Some(elem_id) = get_node_id_from_this(this, ctx) else {
                return Ok(JsValue::undefined());
            };
            let ev_type = args.get_or_undefined(0).to_string(ctx)?;
            let ev_type = ev_type.to_std_string_escaped();
            let callback_val = args.get_or_undefined(1);
            if let Some(cb_obj) = callback_val.as_object() {
                handle.events.add_listener(elem_id, &ev_type, cb_obj.clone());
            }
            Ok(JsValue::undefined())
        },
        h_add_ev,
    );

    let realm = context.realm().clone();

    // Build raw style object
    let raw_style_obj = ObjectInitializer::new(context)
        .property(js_string!("__nodeId"), raw_id, Attribute::NON_ENUMERABLE)
        .function(style_get_fn, js_string!("getProperty"), 1)
        .function(style_set_fn, js_string!("setProperty"), 2)
        .build();

    // Wrap in Proxy via __createStyleProxy(raw_style_obj)
    let global = context.global_object();
    let create_proxy_fn = global.get(js_string!("__createStyleProxy"), context)?;
    let style_proxy = create_proxy_fn.as_callable().map(|c| {
        c.call(&JsValue::undefined(), &[raw_style_obj.into()], context)
    });
    let style_val = match style_proxy {
        Some(Ok(v)) => v,
        _ => JsValue::undefined(),
    };

    let tc_get_js = get_tc_fn.to_js_function(&realm);
    let tc_set_js = set_tc_fn.to_js_function(&realm);
    let ih_get_js = get_ih_fn.to_js_function(&realm);
    let ih_set_js = set_ih_fn.to_js_function(&realm);
    let val_get_js = get_val_fn.to_js_function(&realm);
    let val_set_js = set_val_fn.to_js_function(&realm);
    let tag_get_js = get_tag_fn.to_js_function(&realm);

    let element = ObjectInitializer::new(context)
        .property(js_string!("__nodeId"), raw_id, Attribute::NON_ENUMERABLE)
        .property(js_string!("style"), style_val, Attribute::all())
        .function(get_attr_fn, js_string!("getAttribute"), 1)
        .function(set_attr_fn, js_string!("setAttribute"), 2)
        .function(rem_attr_fn, js_string!("removeAttribute"), 1)
        .function(append_child_fn, js_string!("appendChild"), 1)
        .function(add_event_listener_fn, js_string!("addEventListener"), 2)
        .accessor(
            js_string!("textContent"),
            Some(tc_get_js),
            Some(tc_set_js),
            Attribute::all(),
        )
        .accessor(
            js_string!("innerHTML"),
            Some(ih_get_js),
            Some(ih_set_js),
            Attribute::all(),
        )
        .accessor(
            js_string!("value"),
            Some(val_get_js),
            Some(val_set_js),
            Attribute::all(),
        )
        .accessor(
            js_string!("tagName"),
            Some(tag_get_js),
            None,
            Attribute::READONLY,
        )
        .build();

    Ok(element)
}

fn get_node_id_from_this(this: &JsValue, ctx: &mut Context) -> Option<NodeId> {
    let obj = this.as_object()?;
    let val = obj.get(js_string!("__nodeId"), ctx).ok()?;
    let raw = val.as_number()? as u32;
    Some(NodeId::from_raw(raw))
}

pub fn collect_text_content(dom: &DomTree, node_id: NodeId) -> String {
    let mut out = String::new();
    collect_text_recursive(dom, node_id, &mut out);
    out
}

fn collect_text_recursive(dom: &DomTree, node_id: NodeId, out: &mut String) {
    for child in dom.children(node_id) {
        if let Some(node) = dom.get_node(child) {
            match node.data {
                NodeData::Text(ref t) => out.push_str(t),
                NodeData::Element(_) => collect_text_recursive(dom, child, out),
                _ => {}
            }
        }
    }
}

pub fn set_text_content(dom: &mut DomTree, node_id: NodeId, text: &str) {
    for child in dom.children(node_id) {
        let _ = dom.detach_node(child);
    }
    if !text.is_empty() {
        let text_node = dom.create_text(text);
        let _ = dom.append_child(node_id, text_node);
    }
}

pub fn serialize_inner_html(dom: &DomTree, node_id: NodeId) -> String {
    let mut out = String::new();
    for child in dom.children(node_id) {
        serialize_node_html(dom, child, &mut out);
    }
    out
}

fn serialize_node_html(dom: &DomTree, node_id: NodeId, out: &mut String) {
    let Some(node) = dom.get_node(node_id) else {
        return;
    };
    match &node.data {
        NodeData::Element(elem) => {
            out.push('<');
            out.push_str(&elem.tag_name);
            for (k, v) in &elem.attributes {
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(v);
                out.push('"');
            }
            out.push('>');
            for child in dom.children(node_id) {
                serialize_node_html(dom, child, out);
            }
            out.push_str("</");
            out.push_str(&elem.tag_name);
            out.push('>');
        }
        NodeData::Text(t) => out.push_str(t),
        NodeData::Comment(c) => {
            out.push_str("<!--");
            out.push_str(c);
            out.push_str("-->");
        }
        NodeData::Document => {}
    }
}

pub fn set_inner_html(dom: &mut DomTree, node_id: NodeId, html: &str) {
    for child in dom.children(node_id) {
        let _ = dom.detach_node(child);
    }

    let parsed = parse_html(html);
    let root = parsed.root_id();
    let mut fragment_children = Vec::new();

    let mut body_id = None;
    for n in parsed.nodes() {
        if parsed.tag_name(n).map(|t| t.eq_ignore_ascii_case("body")).unwrap_or(false) {
            body_id = Some(n);
            break;
        }
    }

    if let Some(bid) = body_id {
        fragment_children.extend(parsed.children(bid));
    } else {
        fragment_children.extend(parsed.children(root));
    }

    for child in fragment_children {
        clone_and_append_subtree(&parsed, child, dom, node_id);
    }
}

fn clone_and_append_subtree(
    src_tree: &DomTree,
    src_node: NodeId,
    dst_tree: &mut DomTree,
    dst_parent: NodeId,
) {
    let Some(node) = src_tree.get_node(src_node) else {
        return;
    };

    let new_node_id = match &node.data {
        NodeData::Element(elem) => {
            let nid = dst_tree.create_element(&elem.tag_name);
            for (k, v) in &elem.attributes {
                let _ = dst_tree.set_attribute(nid, k, v);
            }
            nid
        }
        NodeData::Text(text) => dst_tree.create_text(text),
        NodeData::Comment(comm) => dst_tree.create_comment(comm),
        NodeData::Document => return,
    };

    let _ = dst_tree.append_child(dst_parent, new_node_id);

    for child in src_tree.children(src_node) {
        clone_and_append_subtree(src_tree, child, dst_tree, new_node_id);
    }
}

pub fn get_style_property(dom: &DomTree, node_id: NodeId, prop: &str) -> String {
    let prop_kebab = camel_to_kebab(prop);
    let current_style = dom.get_attribute(node_id, "style").unwrap_or("");
    for decl in current_style.split(';') {
        let decl = decl.trim();
        if let Some((k, v)) = decl.split_once(':')
            && k.trim().eq_ignore_ascii_case(&prop_kebab)
        {
            return v.trim().to_string();
        }
    }
    String::new()
}

pub fn set_style_property(dom: &mut DomTree, node_id: NodeId, prop: &str, val: &str) {
    let prop_kebab = camel_to_kebab(prop);
    let current_style = dom.get_attribute(node_id, "style").unwrap_or("").to_string();

    let mut props: Vec<(String, String)> = Vec::new();
    for decl in current_style.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        if let Some((k, v)) = decl.split_once(':') {
            props.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }

    if let Some(existing) = props.iter_mut().find(|(k, _)| k == &prop_kebab) {
        existing.1 = val.to_string();
    } else {
        props.push((prop_kebab, val.to_string()));
    }

    let mut new_style = String::new();
    for (k, v) in props {
        new_style.push_str(&format!("{k}: {v}; "));
    }
    let _ = dom.set_attribute(node_id, "style", new_style.trim());
}

fn camel_to_kebab(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
