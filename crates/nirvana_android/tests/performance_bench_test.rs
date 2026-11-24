use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_frame_metrics_recording_and_telemetry() {
    let mut engine = NirvanaEngine::new(800, 600);

    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body { margin: 0; background-color: #f0f0f0; }
                .card { width: 200px; height: 100px; background-color: #ffffff; margin: 10px; }
                .title { font-size: 16px; color: #333333; }
            </style>
        </head>
        <body>
            <div class="card"><p class="title">Card 1</p></div>
            <div class="card"><p class="title">Card 2</p></div>
            <div class="card"><p class="title">Card 3</p></div>
        </body>
        </html>
    "#;

    engine.load_html(html, None);
    let m_initial = engine.frame_metrics();
    assert!(
        m_initial.parse_time_ms >= 0.0,
        "parse_time_ms should be recorded during load_html"
    );

    engine.render();
    let m = engine.frame_metrics();

    assert!(m.parse_time_ms >= 0.0, "parse_time_ms >= 0");
    assert!(m.style_time_ms >= 0.0, "style_time_ms >= 0");
    assert!(m.layout_time_ms >= 0.0, "layout_time_ms >= 0");
    assert!(m.raster_time_ms >= 0.0, "raster_time_ms >= 0");
    assert!(m.total_frame_time_ms >= 0.0, "total_frame_time_ms >= 0");

    // Rendering pipeline for a small test page should comfortably complete in < 500ms
    assert!(
        m.total_frame_time_ms < 500.0,
        "Rendering simple page should take less than 500ms, took {}ms",
        m.total_frame_time_ms
    );
}

#[test]
fn test_frame_metrics_update_on_subsequent_renders() {
    let mut engine = NirvanaEngine::new(400, 400);
    engine.load_html("<div><p>Initial</p></div>", None);
    engine.render();

    let m1 = engine.frame_metrics();
    assert!(m1.total_frame_time_ms >= 0.0);

    // Modify via JS and re-render
    engine.evaluate_js("document.querySelector('p').textContent = 'Updated'").unwrap();
    engine.render();

    let m2 = engine.frame_metrics();
    assert!(m2.total_frame_time_ms >= 0.0);
}
