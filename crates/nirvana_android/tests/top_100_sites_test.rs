use nirvana_engine::engine::NirvanaEngine;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const SITES: &[&str] = &[
    // 1-20: Docs & Standards
    "https://example.com",
    "http://info.cern.ch",
    "https://www.gnu.org",
    "https://www.ietf.org",
    "https://www.w3.org",
    "https://www.rfc-editor.org",
    "https://www.rust-lang.org",
    "https://doc.rust-lang.org/book/",
    "https://crates.io",
    "https://docs.rs",
    "https://www.python.org",
    "https://docs.python.org/3/",
    "https://go.dev",
    "https://pkg.go.dev",
    "https://nodejs.org",
    "https://developer.mozilla.org/en-US/",
    "https://www.mozilla.org",
    "https://git-scm.com",
    "https://www.sqlite.org",
    "https://www.postgresql.org",
    // 21-35: Linux / OS / Dev Tools
    "https://www.kernel.org",
    "https://www.linux.org",
    "https://archlinux.org",
    "https://www.debian.org",
    "https://ubuntu.com",
    "https://www.freebsd.org",
    "https://ziglang.org",
    "https://llvm.org",
    "https://gcc.gnu.org",
    "https://cmake.org",
    "https://www.docker.com",
    "https://kubernetes.io",
    "https://nginx.org",
    "https://httpd.apache.org",
    "https://redis.io",
    // 36-45: Search & Wikis
    "https://html.duckduckgo.com/html/?q=rust+programming",
    "https://www.bing.com/search?q=rust+programming",
    "https://en.m.wikipedia.org/wiki/Rust_(programming_language)",
    "https://en.wikipedia.org/wiki/World_Wide_Web",
    "https://hi.wikipedia.org",
    "https://simple.wikipedia.org/wiki/Main_Page",
    "https://en.wiktionary.org",
    "https://archive.org",
    "https://www.gutenberg.org",
    "https://arxiv.org",
    // 46-65: News / Tech / Media
    "https://news.ycombinator.com",
    "https://lobste.rs",
    "https://slashdot.org",
    "https://arstechnica.com",
    "https://techcrunch.com",
    "https://www.theverge.com",
    "https://www.wired.com",
    "https://www.bbc.com/news",
    "https://apnews.com",
    "https://www.npr.org",
    "https://www.theguardian.com/international",
    "https://www.aljazeera.com",
    "https://indianexpress.com",
    "https://www.thehindu.com",
    "https://www.ndtv.com",
    "https://www.espn.com",
    "https://www.cricbuzz.com",
    "https://lwn.net",
    "https://www.phoronix.com",
    "https://opensource.com",
    // 66-80: AI / Cloud / Dev Platforms
    "https://www.anthropic.com",
    "https://docs.anthropic.com",
    "https://openai.com",
    "https://huggingface.co",
    "https://github.com/rust-lang/rust",
    "https://about.gitlab.com",
    "https://codeberg.org",
    "https://stackoverflow.com",
    "https://superuser.com",
    "https://dev.to",
    "https://medium.com",
    "https://www.npmjs.com",
    "https://pypi.org",
    "https://packagist.org",
    "https://rubygems.org",
    // 81-100: Education / Gov / Consumer / Classic Web
    "https://www.mit.edu",
    "https://www.stanford.edu",
    "https://www.harvard.edu",
    "https://www.nasa.gov",
    "https://www.nih.gov",
    "https://www.who.int",
    "https://www.un.org/en/",
    "https://www.weather.gov",
    "https://time.is",
    "https://www.worldometers.info",
    "https://www.imdb.com",
    "https://www.goodreads.com",
    "https://www.craigslist.org",
    "https://old.reddit.com",
    "https://httpbin.org/html",
    "https://motherfuckingwebsite.com",
    "https://txti.es",
    "http://neverssl.com",
    "https://www.iana.org",
    "https://www.icann.org",
];

#[derive(Debug)]
struct SiteReport {
    idx: usize,
    url: &'static str,
    ok: bool,
    panicked: bool,
    elapsed_ms: u128,
    text_len: usize,
    rendered_non_bg: usize,
    title: String,
    err: String,
}

#[test]
#[ignore = "Live network stress test across 100 real websites"]
fn test_load_and_render_100_real_websites() {
    assert_eq!(SITES.len(), 100, "Must test exactly 100 distinct websites");

    let queue = Arc::new(Mutex::new((0usize, Vec::<SiteReport>::new())));
    let workers = 8usize;

    std::thread::scope(|s| {
        for _ in 0..workers {
            let q = Arc::clone(&queue);
            s.spawn(move || loop {
                let next = {
                    let mut guard = q.lock().unwrap();
                    if guard.0 >= SITES.len() {
                        None
                    } else {
                        let i = guard.0;
                        guard.0 += 1;
                        Some((i, SITES[i]))
                    }
                };
                let Some((idx, url)) = next else { break };

                let started = Instant::now();
                let res = catch_unwind(AssertUnwindSafe(|| {
                    let mut engine = NirvanaEngine::new(1080, 2400);
                    engine.set_scale_factor(2.75);
                    let nav_res = engine.navigate(url);
                    let elapsed_ms = started.elapsed().as_millis();
                    match nav_res {
                        Ok(()) => {
                            let snippet = engine.page_text_snippet(2048);
                            let title = engine.page_title().unwrap_or("").to_string();
                            let px = engine.pixels();
                            // Count pixels differing from corner background pixel
                            let bg = &px[0..4];
                            let mut non_bg = 0usize;
                            for chunk in px.chunks_exact(4).step_by(8) {
                                if chunk != bg {
                                    non_bg += 1;
                                }
                            }
                            // Also exercise scroll & find_in_page & reader extraction to catch panics!
                            engine.scroll_by(0.0, 200.0);
                            let _ = engine.find_in_page("the", 0);
                            let _ = engine.extract_reader_html();

                            SiteReport {
                                idx: idx + 1,
                                url,
                                ok: true,
                                panicked: false,
                                elapsed_ms,
                                text_len: snippet.len(),
                                rendered_non_bg: non_bg,
                                title,
                                err: String::new(),
                            }
                        }
                        Err(e) => SiteReport {
                            idx: idx + 1,
                            url,
                            ok: false,
                            panicked: false,
                            elapsed_ms,
                            text_len: 0,
                            rendered_non_bg: 0,
                            title: String::new(),
                            err: e,
                        },
                    }
                }));

                let report = match res {
                    Ok(r) => r,
                    Err(_) => SiteReport {
                        idx: idx + 1,
                        url,
                        ok: false,
                        panicked: true,
                        elapsed_ms: started.elapsed().as_millis(),
                        text_len: 0,
                        rendered_non_bg: 0,
                        title: String::new(),
                        err: "RUST PANIC".to_string(),
                    },
                };

                println!(
                    "[{:03}/100] {} | {}ms | text={}px_diff={} | {} | {}",
                    report.idx,
                    if report.ok { "OK  " } else if report.panicked { "PANIC" } else { "FAIL" },
                    report.elapsed_ms,
                    report.text_len,
                    report.rendered_non_bg,
                    report.url,
                    if report.ok { &report.title } else { &report.err }
                );

                let mut guard = q.lock().unwrap();
                guard.1.push(report);
            });
        }
    });

    let mut reports = std::mem::take(&mut queue.lock().unwrap().1);
    reports.sort_by_key(|r| r.idx);

    let ok_count = reports.iter().filter(|r| r.ok).count();
    let panic_count = reports.iter().filter(|r| r.panicked).count();
    let fail_count = reports.iter().filter(|r| !r.ok).count();
    let empty_visual = reports
        .iter()
        .filter(|r| r.ok && r.rendered_non_bg == 0 && r.text_len > 0)
        .count();

    println!("\n=== 100-WEBSITE BENCHMARK SUMMARY ===");
    println!("Loaded OK: {ok_count}/100 | Failed: {fail_count} | Panics: {panic_count} | Blank renders: {empty_visual}");
    for r in reports.iter().filter(|r| !r.ok) {
        println!("  FAILED [{:03}] {} ({}ms): {}", r.idx, r.url, r.elapsed_ms, r.err);
    }

    assert_eq!(panic_count, 0, "Zero Rust panics allowed across 100 sites");
}
