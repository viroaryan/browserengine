use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

#[test]
fn test_table_and_inline_flow_render_cleanly_at_mobile_dpi() {
    let mut engine = NirvanaEngine::new(1080, 2400);
    engine.set_scale_factor(2.75);

    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Nirvana SERP Verification</title>
        <style>
            body { background-color: #f8fafc; color: #1f2937; font-size: 16px; }
            .topbar { background-color: #ffffff; padding: 12px 16px; border-bottom: 1px solid #e2e8f0; }
            .card { background-color: #ffffff; margin: 12px; padding: 14px; border: 1px solid #e2e8f0; border-radius: 12px; }
            .title { color: #1a0dab; font-size: 18px; font-weight: bold; }
            .url { color: #006621; font-size: 13px; margin-top: 2px; }
            .snippet { color: #475569; font-size: 14px; margin-top: 6px; }
            table { width: 100%; margin: 12px; background-color: #fff7ed; }
            td { padding: 6px; }
        </style>
        <script>
            var hidden = "<div>should not parse as DOM</div>";
            if (1 < 2 && 3 > 1) { console.log(hidden); }
        </script>
        <noscript><style>table,div,span,p{display:none}</style></noscript>
    </head>
    <body>
        <div class="topbar">
            <input value="rust programming language" style="width: 240px;">
            <button>Search</button>
        </div>
        <div class="card">
            <div><a class="title" href="https://www.rust-lang.org">The Rust Programming Language - Clean Fast Systems</a></div>
            <div class="url">https://www.rust-lang.org</div>
            <p class="snippet">A language empowering everyone to build <b>reliable</b> and <b>efficient</b> software with pure memory safety and zero GC overhead.</p>
        </div>
        <table>
            <tr><td>1.</td><td><a href="https://news.ycombinator.com">Show HN: Sovereign Pure-Rust Mobile Browser Engine</a> <small>(github.com)</small></td></tr>
            <tr><td>2.</td><td><a href="https://en.m.wikipedia.org">Rust &ndash; Wikipedia Free Encyclopedia &hellip;</a></td></tr>
            <tr><td>3.</td><td><a href="https://example.com">Mozilla vs Custom Rendering Pipeline Benchmark</a> <small>(nirvana.local)</small></td></tr>
        </table>
        <div class="card">
            <div><a class="title" href="https://html.duckduckgo.com">DuckDuckGo Private Search Engine</a></div>
            <div class="url">https://html.duckduckgo.com/html/</div>
            <p class="snippet">Privacy, simplified. Search with fast server-side rendered HTML and zero tracking scripts.</p>
        </div>
        <div style="height: 1100px; background-color: #ffffff; margin: 12px; padding: 14px; border-radius: 12px;">
            <h3>Extended Scrollable Content Block</h3>
            <p>Bottom verification paragraph inside scrollable mobile document.</p>
        </div>
    </body>
    </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    assert_eq!(engine.page_title(), Some("Nirvana SERP Verification"));

    // Confirm <noscript><style>display:none</style></noscript> did NOT hide cards
    let card_node = engine.find_node_by_class("card").expect("card exists");
    assert!(engine.is_node_in_layout(card_node), "noscript CSS must never hide body");

    // Verify table rows stack vertically without overlapping at y = 0
    let layout = engine.layout_root().expect("layout exists");
    let body = &layout.children[0];
    assert!(body.children.len() >= 5);
    let table_box = &body.children[2];
    let row1 = &table_box.children[0];
    let row2 = &table_box.children[1];
    assert!(
        row2.dimensions.content.y() > row1.dimensions.content.y(),
        "Table rows must stack vertically (row1={}, row2={})",
        row1.dimensions.content.y(),
        row2.dimensions.content.y()
    );
    let title_cell = &row1.children[1];
    assert!(
        title_cell
            .children
            .iter()
            .any(|c| c.text_content.as_deref() == Some(" ")),
        "Inline space separator between <a> and <small> inside <td> must be preserved"
    );

    // Save rasterized high-DPI frame for offline visual inspection
    let preview_dir = std::path::Path::new(r"C:\Users\aryan\.gemini\antigravity\brain\0af6cdb4-b4ba-46de-bb56-059f45362c12\scratch");
    let _ = std::fs::create_dir_all(preview_dir);
    let preview_path = preview_dir.join("serp_preview.png");
    let mut png_canvas = nirvana_gfx::Canvas::new(1080, 2400);
    png_canvas.pixels.copy_from_slice(engine.pixels());
    png_canvas.save_png(&preview_path).expect("save preview png");

    // Verify scale factor is preserved after scrolling
    let pixels_before_scroll: Vec<u8> = engine.pixels().to_vec();
    engine.scroll_by(0.0, 120.0);
    assert_eq!(engine.scale_factor(), 2.75);
    assert_ne!(pixels_before_scroll, engine.pixels());

    // Scroll back to 0 and check find_in_page
    engine.scroll_to(0.0, 0.0);
    let matches = engine.find_in_page("Sovereign", 0);
    assert!(matches >= 1, "find_in_page must match Sovereign");

    // Confirm card background rendered cleanly (not transparent)
    let px = engine.pixels();
    assert_eq!(px.len(), 1080 * 2400 * 4);
    let top_px_idx = (10 * 1080 + 10) * 4;
    let c = Rgba::new(px[top_px_idx], px[top_px_idx + 1], px[top_px_idx + 2], px[top_px_idx + 3]);
    assert!(c.a > 0 && c.r > 200);
}
