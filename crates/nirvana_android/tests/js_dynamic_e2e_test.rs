use nirvana_core::color::Rgba;
use nirvana_engine::engine::NirvanaEngine;

fn pixel_at(engine: &NirvanaEngine, x: u32, y: u32) -> Rgba {
    let (w, _h) = engine.dimensions();
    let idx = ((y * w + x) * 4) as usize;
    let pixels = engine.pixels();
    Rgba::new(pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
}

#[test]
fn test_dynamic_counter_web_app_with_pixel_rendering() {
    let mut engine = NirvanaEngine::new(400, 300);

    // Initial page has a red box (400x100) and a button (100x40)
    let html = r#"
        <html>
            <body style="margin: 0px; padding: 0px;">
                <div id="status-card" style="width: 400px; height: 100px; background-color: #ff0000;">
                    <button id="toggle-btn" style="width: 100px; height: 40px;">Toggle</button>
                </div>
                <script>
                    var btn = document.getElementById('toggle-btn');
                    var card = document.getElementById('status-card');
                    var isGreen = false;
                    btn.addEventListener('click', function() {
                        if (!isGreen) {
                            card.style.backgroundColor = '#00ff00';
                            btn.textContent = 'Active';
                            isGreen = true;
                        } else {
                            card.style.backgroundColor = '#0000ff';
                            btn.textContent = 'Inactive';
                            isGreen = false;
                        }
                    });
                </script>
            </body>
        </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    // 1. Initial State: card is Red (#ff0000) at (x: 200, y: 50)
    let initial_color = pixel_at(&engine, 200, 50);
    assert_eq!(initial_color, Rgba::new(255, 0, 0, 255));

    // 2. Click button at (x: 20, y: 20) inside the button area
    let hit = engine.handle_click(20.0, 20.0);
    assert!(hit.is_some());

    // Card should now be Green (#00ff00) at (x: 200, y: 50)
    let green_color = pixel_at(&engine, 200, 50);
    assert_eq!(green_color, Rgba::new(0, 255, 0, 255));

    // Button text in DOM should be "Active"
    let dom1 = engine.dom().expect("dom exists");
    let btn_id = dom1
        .nodes()
        .find(|&n| dom1.get_attribute(n, "id") == Some("toggle-btn"))
        .expect("btn node");
    let mut btn_text = String::new();
    for child in dom1.children(btn_id) {
        if let Some(n) = dom1.get_node(child)
            && let nirvana_dom::NodeData::Text(ref t) = n.data
        {
            btn_text.push_str(t);
        }
    }
    assert_eq!(btn_text, "Active");

    // 3. Click button again -> toggle to Blue (#0000ff)
    let _ = engine.handle_click(20.0, 20.0);
    let blue_color = pixel_at(&engine, 200, 50);
    assert_eq!(blue_color, Rgba::new(0, 0, 255, 255));
}

#[test]
fn test_dynamic_dom_tree_mutation_inner_html_and_layout() {
    let mut engine = NirvanaEngine::new(400, 300);

    let html = r#"
        <html>
            <body style="margin: 0px; padding: 0px;">
                <div id="container" style="width: 400px; height: 50px; background-color: #ffffff;">
                    <span>Placeholder</span>
                </div>
            </body>
        </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    // Use evaluate_js to dynamically replace container content with styled children
    let res = engine.evaluate_js(
        r#"
        var c = document.getElementById('container');
        c.innerHTML = '<div id="child1" style="height: 100px; background-color: #ff00ff;">Child 1</div><div id="child2" style="height: 50px; background-color: #00ffff;">Child 2</div>';
    "#,
    );
    assert!(res.is_ok());

    // Total content height of container has expanded to 150px
    let dom = engine.dom().expect("dom exists");
    let child1 = dom
        .nodes()
        .find(|&n| dom.get_attribute(n, "id") == Some("child1"));
    assert!(child1.is_some());

    let child2 = dom
        .nodes()
        .find(|&n| dom.get_attribute(n, "id") == Some("child2"));
    assert!(child2.is_some());

    // child1 at (x: 50, y: 50) should be magenta (#ff00ff)
    let p1 = pixel_at(&engine, 50, 50);
    assert_eq!(p1, Rgba::new(255, 0, 255, 255));

    // child2 at (x: 50, y: 125) should be cyan (#00ffff)
    let p2 = pixel_at(&engine, 50, 125);
    assert_eq!(p2, Rgba::new(0, 255, 255, 255));
}

#[test]
fn test_dynamic_timer_loop_updates_framebuffer() {
    let mut engine = NirvanaEngine::new(400, 300);

    let html = r#"
        <html>
            <body style="margin: 0px; padding: 0px;">
                <div id="animated-box" style="width: 400px; height: 100px; background-color: #111111;"></div>
                <script>
                    var box = document.getElementById('animated-box');
                    var ticks = 0;
                    setInterval(function() {
                        ticks++;
                        if (ticks === 1) {
                            box.style.backgroundColor = '#ff0000';
                        } else if (ticks === 2) {
                            box.style.backgroundColor = '#00ff00';
                        }
                    }, 50);
                </script>
            </body>
        </html>
    "#;

    engine.load_html(html, None);
    engine.render();

    // Initially #111111
    let p0 = pixel_at(&engine, 100, 50);
    assert_eq!(p0, Rgba::new(17, 17, 17, 255));

    // Advance 50ms -> 1 tick fires -> turns red (#ff0000)
    let fired1 = engine.step_timers(50);
    assert_eq!(fired1, 1);
    let p1 = pixel_at(&engine, 100, 50);
    assert_eq!(p1, Rgba::new(255, 0, 0, 255));

    // Advance 50ms -> 2nd tick fires -> turns green (#00ff00)
    let fired2 = engine.step_timers(50);
    assert_eq!(fired2, 1);
    let p2 = pixel_at(&engine, 100, 50);
    assert_eq!(p2, Rgba::new(0, 255, 0, 255));
}
