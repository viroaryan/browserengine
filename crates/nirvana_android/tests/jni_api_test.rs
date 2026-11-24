use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_engine_heap_pointer_lifecycle() {
    let engine = Box::new(NirvanaEngine::new(300, 300));
    let raw_ptr = Box::into_raw(engine);

    assert!(!raw_ptr.is_null());

    // Verify pointer can be safely dereferenced
    unsafe {
        let engine_ref = raw_ptr.as_mut().expect("valid pointer");
        assert_eq!(engine_ref.dimensions(), (300, 300));

        engine_ref.load_html("<div>Hello</div>", None);
        engine_ref.render();
        assert_eq!(engine_ref.pixels().len(), 300 * 300 * 4);
    }

    // Free heap memory
    unsafe {
        let _ = Box::from_raw(raw_ptr);
    }
}

#[test]
fn test_null_pointer_safety() {
    let null_ptr: *mut NirvanaEngine = std::ptr::null_mut();
    assert!(unsafe { null_ptr.as_mut() }.is_none());
}

#[test]
fn test_engine_pointer_input_lifecycle() {
    let engine = Box::new(NirvanaEngine::new(400, 300));
    let raw_ptr = Box::into_raw(engine);

    unsafe {
        let engine_ref = raw_ptr.as_mut().expect("valid pointer");
        engine_ref.load_html("<input value=\"init\">", None);
        engine_ref.render();

        assert!(!engine_ref.is_input_focused());

        // Focus and edit
        let hit = engine_ref.handle_click(10.0, 10.0);
        assert!(hit.is_some());
        assert!(engine_ref.is_input_focused());

        engine_ref.set_cursor_index(4);
        engine_ref.insert_text("_added");
        assert_eq!(engine_ref.focused_text().as_deref(), Some("init_added"));

        engine_ref.delete_backward();
        assert_eq!(engine_ref.focused_text().as_deref(), Some("init_adde"));

        engine_ref.blur();
        assert!(!engine_ref.is_input_focused());

        let _ = Box::from_raw(raw_ptr);
    }
}

#[test]
fn test_engine_pointer_security_lifecycle() {
    let engine = Box::new(NirvanaEngine::new(400, 300));
    let raw_ptr = Box::into_raw(engine);

    unsafe {
        let engine_ref = raw_ptr.as_mut().expect("valid pointer");
        engine_ref.load_html("<p>Security Test</p>", None);

        let _ = engine_ref.evaluate_js("localStorage.setItem('k', 'v');");
        assert_eq!(
            engine_ref.evaluate_js("localStorage.getItem('k')").as_deref(),
            Ok("v")
        );

        engine_ref.clear_storage();
        assert_eq!(
            engine_ref.evaluate_js("localStorage.getItem('k')").as_deref(),
            Ok("null")
        );

        engine_ref.clear_cookies();
        assert_eq!(engine_ref.cookie_jar().borrow().all_cookies().len(), 0);

        let _ = Box::from_raw(raw_ptr);
    }
}
