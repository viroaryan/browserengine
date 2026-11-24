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
